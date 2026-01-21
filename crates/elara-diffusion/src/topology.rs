//! Topology - The shape of state propagation network
//!
//! How state flows through the swarm. This is NOT a CDN topology.
//! It's a dynamic, interest-driven propagation graph.

use elara_core::NodeId;
use std::collections::{HashMap, HashSet};

/// Edge in the propagation graph
#[derive(Debug, Clone, Copy)]
pub struct PropagationEdge {
    /// Source node
    pub from: NodeId,
    /// Destination node
    pub to: NodeId,
    /// Estimated latency in milliseconds
    pub latency_ms: u32,
    /// Bandwidth capacity (relative, 0-100)
    pub bandwidth: u8,
    /// Is this edge active?
    pub active: bool,
}

impl PropagationEdge {
    pub fn new(from: NodeId, to: NodeId) -> Self {
        Self {
            from,
            to,
            latency_ms: 50,
            bandwidth: 100,
            active: true,
        }
    }
    
    pub fn with_latency(mut self, latency_ms: u32) -> Self {
        self.latency_ms = latency_ms;
        self
    }
    
    pub fn with_bandwidth(mut self, bandwidth: u8) -> Self {
        self.bandwidth = bandwidth;
        self
    }
}

/// Propagation topology
#[derive(Debug, Clone, Default)]
pub struct PropagationTopology {
    /// All nodes in the topology
    nodes: HashSet<NodeId>,
    
    /// Edges: from_node -> list of edges
    edges: HashMap<NodeId, Vec<PropagationEdge>>,
    
    /// Reverse edges: to_node -> list of from_nodes
    reverse_edges: HashMap<NodeId, HashSet<NodeId>>,
}

impl PropagationTopology {
    /// Create a new empty topology
    pub fn new() -> Self {
        Self::default()
    }
    
    /// Add a node
    pub fn add_node(&mut self, node: NodeId) {
        self.nodes.insert(node);
    }
    
    /// Remove a node and all its edges
    pub fn remove_node(&mut self, node: NodeId) {
        self.nodes.remove(&node);
        self.edges.remove(&node);
        
        // Remove edges pointing to this node
        for edges in self.edges.values_mut() {
            edges.retain(|e| e.to != node);
        }
        
        self.reverse_edges.remove(&node);
        for sources in self.reverse_edges.values_mut() {
            sources.remove(&node);
        }
    }
    
    /// Add an edge
    pub fn add_edge(&mut self, edge: PropagationEdge) {
        self.nodes.insert(edge.from);
        self.nodes.insert(edge.to);
        
        self.edges.entry(edge.from).or_default().push(edge);
        self.reverse_edges.entry(edge.to).or_default().insert(edge.from);
    }
    
    /// Get all edges from a node
    pub fn edges_from(&self, node: NodeId) -> &[PropagationEdge] {
        self.edges.get(&node).map(|v| v.as_slice()).unwrap_or(&[])
    }
    
    /// Get all nodes that can receive from a node
    pub fn downstream(&self, node: NodeId) -> Vec<NodeId> {
        self.edges_from(node).iter().map(|e| e.to).collect()
    }
    
    /// Get all nodes that can send to a node
    pub fn upstream(&self, node: NodeId) -> Vec<NodeId> {
        self.reverse_edges
            .get(&node)
            .map(|s| s.iter().copied().collect())
            .unwrap_or_default()
    }
    
    /// Get node count
    pub fn node_count(&self) -> usize {
        self.nodes.len()
    }
    
    /// Check if a node exists
    pub fn has_node(&self, node: NodeId) -> bool {
        self.nodes.contains(&node)
    }
}

/// Star topology - one central node (broadcaster) to all viewers
#[derive(Debug, Clone)]
pub struct StarTopology {
    /// Central node (broadcaster)
    pub center: NodeId,
    /// Leaf nodes (viewers)
    pub leaves: HashSet<NodeId>,
    /// The underlying topology
    pub topology: PropagationTopology,
}

impl StarTopology {
    /// Create a new star topology
    pub fn new(center: NodeId) -> Self {
        let mut topology = PropagationTopology::new();
        topology.add_node(center);
        
        Self {
            center,
            leaves: HashSet::new(),
            topology,
        }
    }
    
    /// Add a leaf (viewer)
    pub fn add_leaf(&mut self, leaf: NodeId) {
        self.leaves.insert(leaf);
        self.topology.add_edge(PropagationEdge::new(self.center, leaf));
    }
    
    /// Remove a leaf
    pub fn remove_leaf(&mut self, leaf: NodeId) {
        self.leaves.remove(&leaf);
        self.topology.remove_node(leaf);
    }
    
    /// Get leaf count
    pub fn leaf_count(&self) -> usize {
        self.leaves.len()
    }
}

/// Tree topology - hierarchical relay (P2P CDN-like)
#[derive(Debug, Clone)]
pub struct TreeTopology {
    /// Root node (broadcaster)
    pub root: NodeId,
    /// Parent of each node
    parents: HashMap<NodeId, NodeId>,
    /// Children of each node
    children: HashMap<NodeId, HashSet<NodeId>>,
    /// Maximum children per node (fan-out)
    pub max_fanout: usize,
    /// The underlying topology
    pub topology: PropagationTopology,
}

impl TreeTopology {
    /// Create a new tree topology
    pub fn new(root: NodeId, max_fanout: usize) -> Self {
        let mut topology = PropagationTopology::new();
        topology.add_node(root);
        
        Self {
            root,
            parents: HashMap::new(),
            children: HashMap::new(),
            max_fanout,
            topology,
        }
    }
    
    /// Add a node to the tree (finds best parent)
    pub fn add_node(&mut self, node: NodeId) -> NodeId {
        // Find a parent with room
        let parent = self.find_parent();
        
        self.parents.insert(node, parent);
        self.children.entry(parent).or_default().insert(node);
        self.topology.add_edge(PropagationEdge::new(parent, node));
        
        parent
    }
    
    /// Find a parent with room for more children
    fn find_parent(&self) -> NodeId {
        // Start with root
        let root_children = self.children.get(&self.root).map(|c| c.len()).unwrap_or(0);
        if root_children < self.max_fanout {
            return self.root;
        }
        
        // BFS to find a node with room
        let mut queue: Vec<NodeId> = self.children
            .get(&self.root)
            .map(|c| c.iter().copied().collect())
            .unwrap_or_default();
        
        while let Some(node) = queue.pop() {
            let child_count = self.children.get(&node).map(|c| c.len()).unwrap_or(0);
            if child_count < self.max_fanout {
                return node;
            }
            
            if let Some(children) = self.children.get(&node) {
                queue.extend(children.iter().copied());
            }
        }
        
        // Fallback to root
        self.root
    }
    
    /// Remove a node (and reassign its children)
    pub fn remove_node(&mut self, node: NodeId) {
        if node == self.root {
            return; // Can't remove root
        }
        
        // Get parent and children
        let parent = self.parents.remove(&node);
        let children = self.children.remove(&node).unwrap_or_default();
        
        // Remove from parent's children
        if let Some(p) = parent {
            if let Some(siblings) = self.children.get_mut(&p) {
                siblings.remove(&node);
            }
        }
        
        // Reassign children to parent or find new parents
        for child in children {
            if let Some(p) = parent {
                self.parents.insert(child, p);
                self.children.entry(p).or_default().insert(child);
                self.topology.add_edge(PropagationEdge::new(p, child));
            }
        }
        
        self.topology.remove_node(node);
    }
    
    /// Get depth of a node
    pub fn depth(&self, node: NodeId) -> usize {
        let mut depth = 0;
        let mut current = node;
        
        while let Some(&parent) = self.parents.get(&current) {
            depth += 1;
            current = parent;
        }
        
        depth
    }
    
    /// Get total node count
    pub fn node_count(&self) -> usize {
        self.topology.node_count()
    }
}

/// Mesh topology - fully connected (for small groups)
#[derive(Debug, Clone)]
pub struct MeshTopology {
    /// All nodes
    nodes: HashSet<NodeId>,
    /// The underlying topology
    pub topology: PropagationTopology,
}

impl MeshTopology {
    /// Create a new mesh topology
    pub fn new() -> Self {
        Self {
            nodes: HashSet::new(),
            topology: PropagationTopology::new(),
        }
    }
    
    /// Add a node (connects to all existing nodes)
    pub fn add_node(&mut self, node: NodeId) {
        // Add edges to/from all existing nodes
        for &existing in &self.nodes {
            self.topology.add_edge(PropagationEdge::new(existing, node));
            self.topology.add_edge(PropagationEdge::new(node, existing));
        }
        
        self.nodes.insert(node);
        self.topology.add_node(node);
    }
    
    /// Remove a node
    pub fn remove_node(&mut self, node: NodeId) {
        self.nodes.remove(&node);
        self.topology.remove_node(node);
    }
    
    /// Get node count
    pub fn node_count(&self) -> usize {
        self.nodes.len()
    }
}

impl Default for MeshTopology {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    
    #[test]
    fn test_star_topology() {
        let broadcaster = NodeId::new(1);
        let mut star = StarTopology::new(broadcaster);
        
        star.add_leaf(NodeId::new(2));
        star.add_leaf(NodeId::new(3));
        star.add_leaf(NodeId::new(4));
        
        assert_eq!(star.leaf_count(), 3);
        assert_eq!(star.topology.downstream(broadcaster).len(), 3);
    }
    
    #[test]
    fn test_tree_topology() {
        let root = NodeId::new(1);
        let mut tree = TreeTopology::new(root, 2);
        
        // Add 6 nodes
        for i in 2..=7 {
            tree.add_node(NodeId::new(i));
        }
        
        assert_eq!(tree.node_count(), 7);
        
        // Root should have 2 children (max fanout)
        assert_eq!(tree.topology.downstream(root).len(), 2);
    }
    
    #[test]
    fn test_mesh_topology() {
        let mut mesh = MeshTopology::new();
        
        mesh.add_node(NodeId::new(1));
        mesh.add_node(NodeId::new(2));
        mesh.add_node(NodeId::new(3));
        
        assert_eq!(mesh.node_count(), 3);
        
        // Each node should be connected to 2 others
        assert_eq!(mesh.topology.downstream(NodeId::new(1)).len(), 2);
    }
}
