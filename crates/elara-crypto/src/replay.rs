//! Replay window management for packet deduplication

use elara_core::{ElaraError, ElaraResult, NodeId, PacketClass};
use std::collections::HashMap;

/// Replay window for a single (node, class) pair
#[derive(Clone, Debug)]
pub struct ReplayWindow {
    /// Minimum accepted sequence number
    min_seq: u16,
    /// Bitmap of received packets in window
    bitmap: u64,
    /// Window size (from packet class)
    window_size: u16,
}

impl ReplayWindow {
    /// Create a new replay window
    pub fn new(window_size: u16) -> Self {
        ReplayWindow {
            min_seq: 0,
            bitmap: 0,
            window_size: window_size.min(64), // Max 64 bits in bitmap
        }
    }

    /// Check if a sequence number is valid (not a replay)
    pub fn check(&self, seq: u16) -> bool {
        // Use wrapping subtraction to handle wraparound
        let offset = seq.wrapping_sub(self.min_seq);

        // If offset is very large (> 32768), it means seq is "before" min_seq
        // considering wraparound (negative in signed interpretation)
        if offset > 32768 {
            // Too old (wrapped around behind min_seq)
            return false;
        }

        if offset >= self.window_size {
            // Ahead of window - always valid
            return true;
        }

        // Check bitmap
        let bit = 1u64 << offset;
        (self.bitmap & bit) == 0
    }

    /// Mark a sequence number as received
    /// Returns true if accepted, false if replay
    pub fn accept(&mut self, seq: u16) -> bool {
        if !self.check(seq) {
            return false;
        }

        let offset = seq.wrapping_sub(self.min_seq);

        if offset >= self.window_size {
            // Advance window
            let advance = offset - self.window_size + 1;
            if advance >= 64 {
                // Large jump - reset bitmap
                self.bitmap = 0;
            } else {
                self.bitmap >>= advance;
            }
            self.min_seq = seq.wrapping_sub(self.window_size - 1);
        }

        // Mark as received
        let new_offset = seq.wrapping_sub(self.min_seq);
        if new_offset < 64 {
            self.bitmap |= 1u64 << new_offset;
        }

        true
    }

    /// Get current minimum sequence
    pub fn min_seq(&self) -> u16 {
        self.min_seq
    }
}

/// Replay protection manager for all peers and classes
#[derive(Debug, Default)]
pub struct ReplayManager {
    /// Windows indexed by (node_id, class)
    windows: HashMap<(NodeId, PacketClass), ReplayWindow>,
}

impl ReplayManager {
    pub fn new() -> Self {
        ReplayManager {
            windows: HashMap::new(),
        }
    }

    /// Check if a packet is valid (not a replay)
    pub fn check(&self, node: NodeId, class: PacketClass, seq: u16) -> bool {
        self.windows
            .get(&(node, class))
            .is_none_or(|w| w.check(seq))
    }

    /// Accept a packet (mark as received)
    /// Returns error if replay detected
    pub fn accept(&mut self, node: NodeId, class: PacketClass, seq: u16) -> ElaraResult<()> {
        let window = self
            .windows
            .entry((node, class))
            .or_insert_with(|| ReplayWindow::new(class.replay_window_size()));

        if window.accept(seq) {
            Ok(())
        } else {
            Err(ElaraError::ReplayDetected(seq as u32))
        }
    }

    /// Remove windows for a node (on disconnect)
    pub fn remove_node(&mut self, node: NodeId) {
        self.windows.retain(|(n, _), _| *n != node);
    }

    /// Get window for inspection
    pub fn get_window(&self, node: NodeId, class: PacketClass) -> Option<&ReplayWindow> {
        self.windows.get(&(node, class))
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_replay_window_basic() {
        let mut window = ReplayWindow::new(16);

        // First packet should be accepted
        assert!(window.accept(0));

        // Replay should be rejected
        assert!(!window.accept(0));

        // Next packet should be accepted
        assert!(window.accept(1));
    }

    #[test]
    fn test_replay_window_out_of_order() {
        let mut window = ReplayWindow::new(16);

        // Accept packets out of order
        assert!(window.accept(5));
        assert!(window.accept(3));
        assert!(window.accept(7));
        assert!(window.accept(1));

        // All should be marked as received
        assert!(!window.accept(5));
        assert!(!window.accept(3));
        assert!(!window.accept(7));
        assert!(!window.accept(1));

        // Gaps should still be valid
        assert!(window.accept(2));
        assert!(window.accept(4));
        assert!(window.accept(6));
    }

    #[test]
    fn test_replay_window_advance() {
        let mut window = ReplayWindow::new(16);

        // Accept some packets
        assert!(window.accept(0));
        assert!(window.accept(1));
        assert!(window.accept(2));

        // Jump ahead
        assert!(window.accept(20));

        // Old packets should now be rejected (too old)
        assert!(!window.accept(0));
        assert!(!window.accept(1));

        // Recent packets should still work
        assert!(window.accept(18));
        assert!(window.accept(19));
    }

    #[test]
    fn test_replay_window_wraparound() {
        let mut window = ReplayWindow::new(16);

        // Near wraparound point
        window.accept(65530);
        window.accept(65535);

        // Wraparound
        assert!(window.accept(0));
        assert!(window.accept(1));

        // Old ones should be rejected
        assert!(!window.accept(65530));
    }

    #[test]
    fn test_replay_manager() {
        let mut manager = ReplayManager::new();
        let node1 = NodeId::new(1);
        let node2 = NodeId::new(2);

        // Different nodes have independent windows
        assert!(manager.accept(node1, PacketClass::Core, 0).is_ok());
        assert!(manager.accept(node2, PacketClass::Core, 0).is_ok());

        // Same node, same class - replay
        assert!(manager.accept(node1, PacketClass::Core, 0).is_err());

        // Same node, different class - OK
        assert!(manager.accept(node1, PacketClass::Perceptual, 0).is_ok());
    }
}
