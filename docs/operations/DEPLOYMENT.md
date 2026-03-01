# ELARA Protocol Deployment Guide

**Version**: 0.2.0  
**Last Updated**: 2024-01  
**Status**: Production  
**Audience**: DevOps Engineers, SREs, Platform Teams

---

## Table of Contents

1. [Overview](#overview)
2. [Prerequisites](#prerequisites)
3. [Deployment Methods](#deployment-methods)
4. [Configuration Management](#configuration-management)
5. [Zero-Downtime Deployments](#zero-downtime-deployments)
6. [Rollback Procedures](#rollback-procedures)
7. [Post-Deployment Validation](#post-deployment-validation)
8. [Troubleshooting](#troubleshooting)

---

## Overview

This guide provides comprehensive instructions for deploying ELARA Protocol nodes in production environments. It covers multiple deployment methods (bare metal, containers, Kubernetes), configuration management, zero-downtime deployment strategies, and rollback procedures.

### Deployment Architecture

ELARA Protocol supports flexible deployment topologies:

- **Standalone**: Single node for development/testing
- **Small Cluster**: 3-10 nodes for small deployments
- **Medium Cluster**: 10-100 nodes for medium deployments
- **Large Cluster**: 100-1000+ nodes for large-scale deployments

### Key Characteristics

- **Stateless Runtime**: Nodes can be restarted without data loss (state is distributed)
- **Peer-to-Peer**: No central coordinator or single point of failure
- **UDP-Based**: Requires UDP connectivity between nodes
- **Graceful Shutdown**: Supports connection draining and clean shutdown
- **Rolling Updates**: Supports zero-downtime upgrades

---

## Prerequisites

### System Requirements


#### Minimum Requirements

| Resource | Minimum | Recommended | Large Deployment |
|----------|---------|-------------|------------------|
| CPU | 1 core | 2 cores | 4+ cores |
| Memory | 1 GB | 2 GB | 4+ GB |
| Disk | 1 GB | 10 GB | 100+ GB |
| Network | 1 Mbps | 10 Mbps | 100+ Mbps |

#### Operating System Support

- **Linux**: Ubuntu 20.04+, Debian 11+, RHEL 8+, CentOS 8+, Amazon Linux 2
- **Container**: Docker 20.10+, containerd 1.6+
- **Orchestration**: Kubernetes 1.21+

### Network Requirements

#### Ports

| Port | Protocol | Purpose | Required |
|------|----------|---------|----------|
| 7777 | UDP | ELARA Protocol communication | Yes |
| 8080 | TCP | Health check HTTP endpoint | Recommended |
| 9090 | TCP | Prometheus metrics endpoint | Recommended |

**Note**: Port 7777 is the default protocol port and can be customized in configuration.

#### Firewall Rules

```bash
# Allow ELARA protocol traffic (UDP)
iptables -A INPUT -p udp --dport 7777 -j ACCEPT
iptables -A OUTPUT -p udp --dport 7777 -j ACCEPT

# Allow health checks (TCP)
iptables -A INPUT -p tcp --dport 8080 -j ACCEPT

# Allow metrics scraping (TCP)
iptables -A INPUT -p tcp --dport 9090 -j ACCEPT

# Save rules
iptables-save > /etc/iptables/rules.v4
```

#### Network Connectivity

- **Peer-to-Peer**: All nodes must be able to reach each other via UDP
- **MTU**: Minimum 1500 bytes recommended (1200 bytes minimum)
- **Latency**: < 100ms between nodes recommended for optimal performance
- **Bandwidth**: 1 Mbps per 100 messages/second sustained load

### Software Dependencies

#### Runtime Dependencies

```bash
# Ubuntu/Debian
apt-get update
apt-get install -y \
    ca-certificates \
    libssl3 \
    chrony

# RHEL/CentOS
yum install -y \
    ca-certificates \
    openssl-libs \
    chrony

# Start time synchronization
systemctl enable chronyd
systemctl start chronyd
```

#### Time Synchronization

**Critical**: ELARA Protocol requires accurate time synchronization across all nodes.

```bash
# Configure chrony for production
cat > /etc/chrony/chrony.conf <<EOF
# Use multiple reliable time sources
server time.google.com iburst
server time.cloudflare.com iburst
server time.nist.gov iburst

# Allow larger time steps during startup
makestep 1.0 3

# Enable kernel synchronization
rtcsync

# Log directory
logdir /var/log/chrony
EOF

# Restart chrony
systemctl restart chronyd

# Verify synchronization
chronyc tracking
# Expected: System time within 50ms of reference
```

---

## Deployment Methods

### Method 1: Bare Metal / VM Deployment

Best for: Traditional infrastructure, maximum performance, full control.

#### Step 1: Install Binary

**Option A: Download Pre-built Binary**

```bash
# Download latest release
VERSION="1.0.0"
wget https://github.com/elara-protocol/elara/releases/download/v${VERSION}/elara-node-linux-amd64.tar.gz

# Verify checksum
sha256sum -c elara-node-linux-amd64.tar.gz.sha256

# Extract
tar -xzf elara-node-linux-amd64.tar.gz

# Install
sudo mv elara-node /usr/local/bin/
sudo chmod +x /usr/local/bin/elara-node

# Verify installation
elara-node --version
```

**Option B: Build from Source**

```bash
# Install Rust toolchain
curl --proto '=https' --tlsv1.2 -sSf https://sh.rustup.rs | sh
source $HOME/.cargo/env

# Clone repository
git clone https://github.com/elara-protocol/elara.git
cd elara

# Build release binary
cargo build --release --bin elara-node

# Install
sudo cp target/release/elara-node /usr/local/bin/
sudo chmod +x /usr/local/bin/elara-node
```

#### Step 2: Create Configuration

```bash
# Create configuration directory
sudo mkdir -p /etc/elara

# Create configuration file
sudo tee /etc/elara/config.toml <<EOF
# ELARA Node Configuration

[node]
# Unique node identifier (must be unique across cluster)
node_id = "node-1"

# Network bind address
bind_address = "0.0.0.0:7777"

# Peer nodes (other nodes in the cluster)
peers = [
    "node-2.example.com:7777",
    "node-3.example.com:7777",
]

[runtime]
# Tick interval (milliseconds)
tick_interval_ms = 100

# Buffer sizes
max_packet_buffer = 1000
max_outgoing_buffer = 1000
max_local_events = 1000

[observability]
# Enable observability features
enabled = true

[observability.logging]
# Log level: trace, debug, info, warn, error
level = "info"

# Log format: json, pretty, compact
format = "json"

# Log output: stdout, stderr, file, syslog
output = "stdout"

[observability.metrics_server]
# Metrics server bind address
bind_address = "0.0.0.0"
port = 9090

[health_checks]
# Enable health checks
enabled = true

# Health server bind address
server_bind_address = "0.0.0.0:8080"

# Cache TTL for health check results
cache_ttl_secs = 30

# Minimum active connections (null = no check)
min_connections = 2

# Maximum memory usage in MB (null = no check)
max_memory_mb = 1800

# Maximum time drift in milliseconds (null = no check)
max_time_drift_ms = 100

# Maximum pending events (null = no check)
max_pending_events = 1000
EOF

# Set permissions
sudo chmod 600 /etc/elara/config.toml
```

#### Step 3: Create Systemd Service

```bash
# Create systemd service file
sudo tee /etc/systemd/system/elara-node.service <<EOF
[Unit]
Description=ELARA Protocol Node
Documentation=https://github.com/elara-protocol/elara
After=network-online.target chronyd.service
Wants=network-online.target

[Service]
Type=simple
User=elara
Group=elara
ExecStart=/usr/local/bin/elara-node --config /etc/elara/config.toml
ExecReload=/bin/kill -HUP \$MAINPID
Restart=on-failure
RestartSec=5s

# Resource limits
LimitNOFILE=65536
LimitNPROC=4096
MemoryMax=2G
CPUQuota=200%

# Security hardening
NoNewPrivileges=true
PrivateTmp=true
ProtectSystem=strict
ProtectHome=true
ReadWritePaths=/var/lib/elara /var/log/elara

# Graceful shutdown
TimeoutStopSec=30
KillMode=mixed
KillSignal=SIGTERM

[Install]
WantedBy=multi-user.target
EOF

# Create elara user
sudo useradd -r -s /bin/false elara

# Create data directories
sudo mkdir -p /var/lib/elara /var/log/elara
sudo chown -R elara:elara /var/lib/elara /var/log/elara

# Reload systemd
sudo systemctl daemon-reload

# Enable service
sudo systemctl enable elara-node
```

#### Step 4: Start Node

```bash
# Start the service
sudo systemctl start elara-node

# Check status
sudo systemctl status elara-node

# View logs
sudo journalctl -u elara-node -f

# Verify health
curl http://localhost:8080/health | jq

# Verify metrics
curl http://localhost:9090/metrics | grep elara_
```

---

### Method 2: Docker Container Deployment

Best for: Containerized environments, development, testing, CI/CD.

#### Step 1: Create Dockerfile

```dockerfile
# Dockerfile for ELARA Node
FROM rust:1.75 as builder

WORKDIR /build

# Copy source
COPY . .

# Build release binary
RUN cargo build --release --bin elara-node

# Runtime image
FROM debian:bookworm-slim

# Install runtime dependencies
RUN apt-get update && \
    apt-get install -y \
        ca-certificates \
        libssl3 \
        chrony \
        curl \
        jq && \
    rm -rf /var/lib/apt/lists/*

# Create elara user
RUN useradd -r -u 1000 -s /bin/false elara

# Copy binary from builder
COPY --from=builder /build/target/release/elara-node /usr/local/bin/

# Create directories
RUN mkdir -p /etc/elara /var/lib/elara /var/log/elara && \
    chown -R elara:elara /var/lib/elara /var/log/elara

# Expose ports
EXPOSE 7777/udp 8080/tcp 9090/tcp

# Switch to elara user
USER elara

# Health check
HEALTHCHECK --interval=30s --timeout=5s --start-period=10s --retries=3 \
    CMD curl -f http://localhost:8080/health || exit 1

# Start node
ENTRYPOINT ["/usr/local/bin/elara-node"]
CMD ["--config", "/etc/elara/config.toml"]
```

#### Step 2: Build Image

```bash
# Build image
docker build -t elara-node:1.0.0 .

# Tag as latest
docker tag elara-node:1.0.0 elara-node:latest

# Push to registry (optional)
docker tag elara-node:1.0.0 registry.example.com/elara-node:1.0.0
docker push registry.example.com/elara-node:1.0.0
```

#### Step 3: Run Container

```bash
# Create configuration file
mkdir -p ./config
cat > ./config/config.toml <<EOF
# See bare metal configuration example above
EOF

# Run container
docker run -d \
    --name elara-node-1 \
    --restart unless-stopped \
    -p 7777:7777/udp \
    -p 8080:8080/tcp \
    -p 9090:9090/tcp \
    -v $(pwd)/config:/etc/elara:ro \
    -v elara-data:/var/lib/elara \
    -v elara-logs:/var/log/elara \
    --memory=2g \
    --cpus=2 \
    elara-node:1.0.0

# View logs
docker logs -f elara-node-1

# Check health
curl http://localhost:8080/health | jq

# Check metrics
curl http://localhost:9090/metrics | grep elara_
```

#### Step 4: Docker Compose (Multi-Node)

```yaml
# docker-compose.yml
version: '3.8'

services:
  node-1:
    image: elara-node:1.0.0
    container_name: elara-node-1
    restart: unless-stopped
    ports:
      - "7771:7777/udp"
      - "8081:8080/tcp"
      - "9091:9090/tcp"
    volumes:
      - ./config/node-1.toml:/etc/elara/config.toml:ro
      - node-1-data:/var/lib/elara
      - node-1-logs:/var/log/elara
    deploy:
      resources:
        limits:
          cpus: '2'
          memory: 2G
        reservations:
          cpus: '1'
          memory: 1G
    healthcheck:
      test: ["CMD", "curl", "-f", "http://localhost:8080/health"]
      interval: 30s
      timeout: 5s
      retries: 3
      start_period: 10s

  node-2:
    image: elara-node:1.0.0
    container_name: elara-node-2
    restart: unless-stopped
    ports:
      - "7772:7777/udp"
      - "8082:8080/tcp"
      - "9092:9090/tcp"
    volumes:
      - ./config/node-2.toml:/etc/elara/config.toml:ro
      - node-2-data:/var/lib/elara
      - node-2-logs:/var/log/elara
    deploy:
      resources:
        limits:
          cpus: '2'
          memory: 2G
    healthcheck:
      test: ["CMD", "curl", "-f", "http://localhost:8080/health"]
      interval: 30s
      timeout: 5s
      retries: 3

  node-3:
    image: elara-node:1.0.0
    container_name: elara-node-3
    restart: unless-stopped
    ports:
      - "7773:7777/udp"
      - "8083:8080/tcp"
      - "9093:9090/tcp"
    volumes:
      - ./config/node-3.toml:/etc/elara/config.toml:ro
      - node-3-data:/var/lib/elara
      - node-3-logs:/var/log/elara
    deploy:
      resources:
        limits:
          cpus: '2'
          memory: 2G
    healthcheck:
      test: ["CMD", "curl", "-f", "http://localhost:8080/health"]
      interval: 30s
      timeout: 5s
      retries: 3

volumes:
  node-1-data:
  node-1-logs:
  node-2-data:
  node-2-logs:
  node-3-data:
  node-3-logs:
```

```bash
# Start cluster
docker-compose up -d

# View logs
docker-compose logs -f

# Check health of all nodes
for port in 8081 8082 8083; do
    echo "=== Node on port $port ==="
    curl http://localhost:$port/health | jq .status
done

# Stop cluster
docker-compose down
```

---

### Method 3: Kubernetes Deployment

Best for: Cloud-native environments, auto-scaling, high availability.

#### Step 1: Create Namespace

```yaml
# namespace.yaml
apiVersion: v1
kind: Namespace
metadata:
  name: elara-production
  labels:
    name: elara-production
    environment: production
```

```bash
kubectl apply -f namespace.yaml
```

#### Step 2: Create ConfigMap

```yaml
# configmap.yaml
apiVersion: v1
kind: ConfigMap
metadata:
  name: elara-config
  namespace: elara-production
data:
  config.toml: |
    [node]
    # node_id will be set via environment variable (pod name)
    bind_address = "0.0.0.0:7777"
    
    # Peers will be discovered via Kubernetes service
    peers = []
    
    [runtime]
    tick_interval_ms = 100
    max_packet_buffer = 1000
    max_outgoing_buffer = 1000
    max_local_events = 1000
    
    [observability]
    enabled = true
    
    [observability.logging]
    level = "info"
    format = "json"
    output = "stdout"
    
    [observability.metrics_server]
    bind_address = "0.0.0.0"
    port = 9090
    
    [health_checks]
    enabled = true
    server_bind_address = "0.0.0.0:8080"
    cache_ttl_secs = 30
    min_connections = 2
    max_memory_mb = 1800
    max_time_drift_ms = 100
    max_pending_events = 1000
```

```bash
kubectl apply -f configmap.yaml
```

#### Step 3: Create StatefulSet

```yaml
# statefulset.yaml
apiVersion: apps/v1
kind: StatefulSet
metadata:
  name: elara-node
  namespace: elara-production
  labels:
    app: elara-node
spec:
  serviceName: elara-node
  replicas: 3
  selector:
    matchLabels:
      app: elara-node
  template:
    metadata:
      labels:
        app: elara-node
      annotations:
        prometheus.io/scrape: "true"
        prometheus.io/port: "9090"
        prometheus.io/path: "/metrics"
    spec:
      # Anti-affinity to spread pods across nodes
      affinity:
        podAntiAffinity:
          preferredDuringSchedulingIgnoredDuringExecution:
          - weight: 100
            podAffinityTerm:
              labelSelector:
                matchExpressions:
                - key: app
                  operator: In
                  values:
                  - elara-node
              topologyKey: kubernetes.io/hostname
      
      # Service account for peer discovery
      serviceAccountName: elara-node
      
      containers:
      - name: elara-node
        image: registry.example.com/elara-node:1.0.0
        imagePullPolicy: IfNotPresent
        
        # Set node_id from pod name
        env:
        - name: POD_NAME
          valueFrom:
            fieldRef:
              fieldPath: metadata.name
        - name: POD_NAMESPACE
          valueFrom:
            fieldRef:
              fieldPath: metadata.namespace
        - name: NODE_ID
          value: "$(POD_NAME)"
        
        ports:
        - name: protocol
          containerPort: 7777
          protocol: UDP
        - name: health
          containerPort: 8080
          protocol: TCP
        - name: metrics
          containerPort: 9090
          protocol: TCP
        
        # Resource limits
        resources:
          requests:
            cpu: 1000m
            memory: 1Gi
          limits:
            cpu: 2000m
            memory: 2Gi
        
        # Liveness probe
        livenessProbe:
          httpGet:
            path: /live
            port: health
          initialDelaySeconds: 30
          periodSeconds: 10
          timeoutSeconds: 5
          failureThreshold: 3
        
        # Readiness probe
        readinessProbe:
          httpGet:
            path: /ready
            port: health
          initialDelaySeconds: 10
          periodSeconds: 5
          timeoutSeconds: 3
          failureThreshold: 2
        
        # Startup probe (for slow starts)
        startupProbe:
          httpGet:
            path: /live
            port: health
          initialDelaySeconds: 0
          periodSeconds: 5
          timeoutSeconds: 3
          failureThreshold: 12  # 60 seconds total
        
        # Volume mounts
        volumeMounts:
        - name: config
          mountPath: /etc/elara
          readOnly: true
        - name: data
          mountPath: /var/lib/elara
        
        # Security context
        securityContext:
          runAsNonRoot: true
          runAsUser: 1000
          allowPrivilegeEscalation: false
          capabilities:
            drop:
            - ALL
          readOnlyRootFilesystem: true
      
      volumes:
      - name: config
        configMap:
          name: elara-config
  
  # Volume claim templates for persistent storage
  volumeClaimTemplates:
  - metadata:
      name: data
    spec:
      accessModes: [ "ReadWriteOnce" ]
      storageClassName: fast-ssd
      resources:
        requests:
          storage: 10Gi
```

```bash
kubectl apply -f statefulset.yaml
```

#### Step 4: Create Services

```yaml
# service.yaml
---
# Headless service for StatefulSet
apiVersion: v1
kind: Service
metadata:
  name: elara-node
  namespace: elara-production
  labels:
    app: elara-node
spec:
  clusterIP: None
  selector:
    app: elara-node
  ports:
  - name: protocol
    port: 7777
    protocol: UDP
  - name: health
    port: 8080
    protocol: TCP
  - name: metrics
    port: 9090
    protocol: TCP

---
# Service for external access (LoadBalancer)
apiVersion: v1
kind: Service
metadata:
  name: elara-node-external
  namespace: elara-production
  labels:
    app: elara-node
spec:
  type: LoadBalancer
  selector:
    app: elara-node
  ports:
  - name: protocol
    port: 7777
    targetPort: 7777
    protocol: UDP
  sessionAffinity: ClientIP
  sessionAffinityConfig:
    clientIP:
      timeoutSeconds: 3600

---
# Service for metrics scraping
apiVersion: v1
kind: Service
metadata:
  name: elara-metrics
  namespace: elara-production
  labels:
    app: elara-node
  annotations:
    prometheus.io/scrape: "true"
    prometheus.io/port: "9090"
spec:
  selector:
    app: elara-node
  ports:
  - name: metrics
    port: 9090
    targetPort: 9090
    protocol: TCP
```

```bash
kubectl apply -f service.yaml
```

#### Step 5: Create RBAC (for peer discovery)

```yaml
# rbac.yaml
---
apiVersion: v1
kind: ServiceAccount
metadata:
  name: elara-node
  namespace: elara-production

---
apiVersion: rbac.authorization.k8s.io/v1
kind: Role
metadata:
  name: elara-node
  namespace: elara-production
rules:
- apiGroups: [""]
  resources: ["pods", "endpoints"]
  verbs: ["get", "list", "watch"]

---
apiVersion: rbac.authorization.k8s.io/v1
kind: RoleBinding
metadata:
  name: elara-node
  namespace: elara-production
roleRef:
  apiGroup: rbac.authorization.k8s.io
  kind: Role
  name: elara-node
subjects:
- kind: ServiceAccount
  name: elara-node
  namespace: elara-production
```

```bash
kubectl apply -f rbac.yaml
```

#### Step 6: Deploy and Verify

```bash
# Deploy all resources
kubectl apply -f namespace.yaml
kubectl apply -f configmap.yaml
kubectl apply -f rbac.yaml
kubectl apply -f service.yaml
kubectl apply -f statefulset.yaml

# Wait for pods to be ready
kubectl wait --for=condition=ready pod -l app=elara-node -n elara-production --timeout=120s

# Check pod status
kubectl get pods -n elara-production -l app=elara-node

# Check logs
kubectl logs -n elara-production elara-node-0 -f

# Check health of all pods
for i in 0 1 2; do
    echo "=== elara-node-$i ==="
    kubectl exec -n elara-production elara-node-$i -- curl -s http://localhost:8080/health | jq .status
done

# Check metrics
kubectl port-forward -n elara-production elara-node-0 9090:9090 &
curl http://localhost:9090/metrics | grep elara_
```

#### Step 7: Configure Horizontal Pod Autoscaler (Optional)

```yaml
# hpa.yaml
apiVersion: autoscaling/v2
kind: HorizontalPodAutoscaler
metadata:
  name: elara-node
  namespace: elara-production
spec:
  scaleTargetRef:
    apiVersion: apps/v1
    kind: StatefulSet
    name: elara-node
  minReplicas: 3
  maxReplicas: 10
  metrics:
  - type: Resource
    resource:
      name: cpu
      target:
        type: Utilization
        averageUtilization: 70
  - type: Resource
    resource:
      name: memory
      target:
        type: Utilization
        averageUtilization: 80
  behavior:
    scaleDown:
      stabilizationWindowSeconds: 300
      policies:
      - type: Percent
        value: 50
        periodSeconds: 60
    scaleUp:
      stabilizationWindowSeconds: 60
      policies:
      - type: Percent
        value: 100
        periodSeconds: 60
```

```bash
kubectl apply -f hpa.yaml
```

---

## Configuration Management

### Environment-Specific Configuration

#### Development Environment

```toml
# config/dev.toml
[node]
node_id = "dev-node-1"
bind_address = "0.0.0.0:7777"
peers = []

[runtime]
tick_interval_ms = 100
max_packet_buffer = 100
max_outgoing_buffer = 100
max_local_events = 100

[observability]
enabled = true

[observability.logging]
level = "debug"
format = "pretty"
output = "stdout"

[observability.metrics_server]
bind_address = "127.0.0.1"
port = 9090

[health_checks]
enabled = true
server_bind_address = "127.0.0.1:8080"
cache_ttl_secs = 10
min_connections = null  # No minimum in dev
max_memory_mb = null    # No limit in dev
max_time_drift_ms = null
max_pending_events = null
```

#### Staging Environment

```toml
# config/staging.toml
[node]
node_id = "staging-node-1"
bind_address = "0.0.0.0:7777"
peers = [
    "staging-node-2.internal:7777",
    "staging-node-3.internal:7777",
]

[runtime]
tick_interval_ms = 100
max_packet_buffer = 500
max_outgoing_buffer = 500
max_local_events = 500

[observability]
enabled = true

[observability.logging]
level = "info"
format = "json"
output = "stdout"

[observability.metrics_server]
bind_address = "0.0.0.0"
port = 9090

[observability.tracing]
enabled = true
service_name = "elara-node-staging"
exporter = "otlp"
otlp_endpoint = "http://jaeger-collector.monitoring:4317"
sample_rate = 0.1

[health_checks]
enabled = true
server_bind_address = "0.0.0.0:8080"
cache_ttl_secs = 30
min_connections = 1
max_memory_mb = 1800
max_time_drift_ms = 200
max_pending_events = 1000
```

#### Production Environment

```toml
# config/production.toml
[node]
node_id = "prod-node-1"
bind_address = "0.0.0.0:7777"
peers = [
    "prod-node-2.internal:7777",
    "prod-node-3.internal:7777",
    "prod-node-4.internal:7777",
    "prod-node-5.internal:7777",
]

[runtime]
tick_interval_ms = 100
max_packet_buffer = 1000
max_outgoing_buffer = 1000
max_local_events = 1000

[observability]
enabled = true

[observability.logging]
level = "info"
format = "json"
output = "stdout"

[observability.metrics_server]
bind_address = "0.0.0.0"
port = 9090

[observability.tracing]
enabled = true
service_name = "elara-node-production"
exporter = "otlp"
otlp_endpoint = "http://jaeger-collector.monitoring:4317"
sample_rate = 0.01  # 1% sampling in production

[health_checks]
enabled = true
server_bind_address = "0.0.0.0:8080"
cache_ttl_secs = 30
min_connections = 3
max_memory_mb = 1800
max_time_drift_ms = 100
max_pending_events = 1000
```

### Configuration Validation

```bash
# Validate configuration before deployment
elara-node --config /etc/elara/config.toml --validate

# Expected output: "Configuration is valid"

# If validation fails, check:
# - TOML syntax errors
# - Missing required fields
# - Invalid values (e.g., negative numbers)
# - Port conflicts
```

### Secrets Management

#### Using Environment Variables

```bash
# Set sensitive values via environment
export ELARA_NODE_ID="prod-node-1"
export ELARA_CRYPTO_KEY="$(cat /secrets/crypto-key)"

# Reference in config
# node_id = "${ELARA_NODE_ID}"
```

#### Using Kubernetes Secrets

```yaml
# secret.yaml
apiVersion: v1
kind: Secret
metadata:
  name: elara-secrets
  namespace: elara-production
type: Opaque
data:
  crypto-key: <base64-encoded-key>
```

```yaml
# Reference in StatefulSet
env:
- name: ELARA_CRYPTO_KEY
  valueFrom:
    secretKeyRef:
      name: elara-secrets
      key: crypto-key
```

---

## Zero-Downtime Deployments

### Rolling Update Strategy

#### Bare Metal / VM

```bash
# Update nodes one at a time
for node in node-1 node-2 node-3; do
    echo "=== Updating $node ==="
    
    # 1. Check node health
    ssh $node 'curl -f http://localhost:8080/health'
    
    # 2. Download new binary
    ssh $node 'wget -O /tmp/elara-node-new https://releases.example.com/elara-node-1.1.0'
    
    # 3. Verify checksum
    ssh $node 'sha256sum -c /tmp/elara-node-new.sha256'
    
    # 4. Graceful shutdown
    ssh $node 'systemctl stop elara-node'
    
    # 5. Replace binary
    ssh $node 'mv /tmp/elara-node-new /usr/local/bin/elara-node && chmod +x /usr/local/bin/elara-node'
    
    # 6. Start new version
    ssh $node 'systemctl start elara-node'
    
    # 7. Wait for health
    sleep 30
    ssh $node 'curl -f http://localhost:8080/health'
    
    # 8. Verify connections
    ssh $node 'curl -s http://localhost:9090/metrics | grep elara_active_connections'
    
    echo "=== $node updated successfully ==="
    sleep 60  # Wait before next node
done
```

#### Docker

```bash
# Pull new image
docker pull registry.example.com/elara-node:1.1.0

# Update containers one at a time
for container in elara-node-1 elara-node-2 elara-node-3; do
    echo "=== Updating $container ==="
    
    # Stop old container
    docker stop $container
    
    # Remove old container
    docker rm $container
    
    # Start new container
    docker run -d \
        --name $container \
        --restart unless-stopped \
        -p 7777:7777/udp \
        -p 8080:8080/tcp \
        -p 9090:9090/tcp \
        -v $(pwd)/config:/etc/elara:ro \
        registry.example.com/elara-node:1.1.0
    
    # Wait for health
    sleep 30
    docker exec $container curl -f http://localhost:8080/health
    
    echo "=== $container updated successfully ==="
    sleep 60
done
```

#### Kubernetes

```bash
# Update image in StatefulSet
kubectl set image statefulset/elara-node \
    elara-node=registry.example.com/elara-node:1.1.0 \
    -n elara-production

# Or apply updated manifest
kubectl apply -f statefulset.yaml

# Watch rollout
kubectl rollout status statefulset/elara-node -n elara-production

# Verify pods are running new version
kubectl get pods -n elara-production -l app=elara-node -o jsonpath='{range .items[*]}{.metadata.name}{"\t"}{.spec.containers[0].image}{"\n"}{end}'

# Check health of all pods
for i in 0 1 2; do
    kubectl exec -n elara-production elara-node-$i -- curl -s http://localhost:8080/health | jq .status
done
```

### Blue-Green Deployment

```bash
# Deploy green environment
kubectl apply -f statefulset-green.yaml

# Wait for green to be healthy
kubectl wait --for=condition=ready pod -l app=elara-node,version=green -n elara-production

# Switch traffic to green
kubectl patch service elara-node-external -n elara-production -p '{"spec":{"selector":{"version":"green"}}}'

# Monitor for issues
sleep 300

# If successful, remove blue
kubectl delete statefulset elara-node-blue -n elara-production

# If issues, rollback
kubectl patch service elara-node-external -n elara-production -p '{"spec":{"selector":{"version":"blue"}}}'
```

### Canary Deployment

```yaml
# Deploy canary with 10% traffic
apiVersion: apps/v1
kind: StatefulSet
metadata:
  name: elara-node-canary
spec:
  replicas: 1  # 10% of 10 nodes
  # ... rest of spec with new version
```

```bash
# Deploy canary
kubectl apply -f statefulset-canary.yaml

# Monitor canary metrics
kubectl port-forward -n elara-production elara-node-canary-0 9090:9090 &
watch 'curl -s http://localhost:9090/metrics | grep -E "elara_(messages_dropped|message_latency)"'

# If successful, scale up canary and scale down stable
kubectl scale statefulset elara-node-canary --replicas=10 -n elara-production
kubectl scale statefulset elara-node --replicas=0 -n elara-production

# Rename canary to stable
kubectl delete statefulset elara-node -n elara-production
kubectl apply -f statefulset.yaml  # with new version
```

---

## Rollback Procedures

### Quick Rollback (Emergency)

#### Bare Metal / VM

```bash
# Rollback to previous version immediately
for node in node-1 node-2 node-3; do
    echo "=== Rolling back $node ==="
    
    # Stop current version
    ssh $node 'systemctl stop elara-node'
    
    # Restore previous binary
    ssh $node 'cp /usr/local/bin/elara-node.backup /usr/local/bin/elara-node'
    
    # Start previous version
    ssh $node 'systemctl start elara-node'
    
    # Verify health
    sleep 15
    ssh $node 'curl -f http://localhost:8080/health'
done
```

#### Docker

```bash
# Rollback to previous image
for container in elara-node-1 elara-node-2 elara-node-3; do
    docker stop $container
    docker rm $container
    docker run -d \
        --name $container \
        --restart unless-stopped \
        -p 7777:7777/udp \
        -p 8080:8080/tcp \
        -p 9090:9090/tcp \
        -v $(pwd)/config:/etc/elara:ro \
        registry.example.com/elara-node:1.0.0  # Previous version
    
    sleep 15
    docker exec $container curl -f http://localhost:8080/health
done
```

#### Kubernetes

```bash
# Rollback StatefulSet to previous revision
kubectl rollout undo statefulset/elara-node -n elara-production

# Or rollback to specific revision
kubectl rollout undo statefulset/elara-node --to-revision=2 -n elara-production

# Watch rollback progress
kubectl rollout status statefulset/elara-node -n elara-production

# Verify rollback
kubectl get pods -n elara-production -l app=elara-node -o jsonpath='{range .items[*]}{.metadata.name}{"\t"}{.spec.containers[0].image}{"\n"}{end}'
```

### Gradual Rollback

```bash
# Rollback one node at a time (safer)
for i in 0 1 2; do
    echo "=== Rolling back elara-node-$i ==="
    
    # Delete pod to trigger recreation with previous version
    kubectl delete pod elara-node-$i -n elara-production
    
    # Wait for pod to be ready
    kubectl wait --for=condition=ready pod/elara-node-$i -n elara-production --timeout=120s
    
    # Verify health
    kubectl exec -n elara-production elara-node-$i -- curl -s http://localhost:8080/health | jq .status
    
    # Wait before next node
    sleep 60
done
```

### Rollback Verification

```bash
# Check version of all nodes
for node in node-1 node-2 node-3; do
    echo "=== $node ==="
    ssh $node 'elara-node --version'
done

# Check health of all nodes
for node in node-1 node-2 node-3; do
    echo "=== $node ==="
    ssh $node 'curl -s http://localhost:8080/health | jq .status'
done

# Check metrics for errors
for node in node-1 node-2 node-3; do
    echo "=== $node ==="
    ssh $node 'curl -s http://localhost:9090/metrics | grep -E "elara_(messages_dropped|failed_connections)"'
done

# Check logs for errors
for node in node-1 node-2 node-3; do
    echo "=== $node ==="
    ssh $node 'journalctl -u elara-node -n 50 | grep -i error'
done
```

---

## Post-Deployment Validation

### Health Check Validation

```bash
# Check health endpoint
curl http://node-1:8080/health | jq

# Expected output:
# {
#   "status": "healthy",
#   "timestamp": "2024-01-15T10:30:00Z",
#   "checks": {
#     "connections": { "status": "healthy" },
#     "memory": { "status": "healthy" },
#     "time_drift": { "status": "healthy" },
#     "state_convergence": { "status": "healthy" }
#   }
# }

# Check all nodes
for node in node-1 node-2 node-3; do
    echo "=== $node ==="
    curl -s http://$node:8080/health | jq .status
done
```

### Connectivity Validation

```bash
# Check active connections
curl http://node-1:9090/metrics | grep elara_active_connections

# Expected: elara_active_connections >= min_connections (e.g., 2)

# Check connection failures
curl http://node-1:9090/metrics | grep elara_failed_connections_total

# Expected: Low or zero failures

# Test UDP connectivity between nodes
nc -u -zv node-2 7777
nc -u -zv node-3 7777
```

### Performance Validation

```bash
# Check message latency
curl http://node-1:9090/metrics | grep elara_message_latency_ms

# Expected: P95 < 100ms under normal load

# Check message drop rate
curl http://node-1:9090/metrics | grep elara_messages_dropped_total

# Expected: < 0.1% drop rate

# Check memory usage
curl http://node-1:9090/metrics | grep elara_memory_usage_bytes

# Expected: < 1.8GB (90% of 2GB limit)

# Check time drift
curl http://node-1:9090/metrics | grep elara_time_drift_ms

# Expected: < 50ms
```

### Load Testing Validation

```bash
# Run small load test
elara-loadtest --scenario small --duration 60s --target node-1:7777

# Expected results:
# - Success rate > 99%
# - P95 latency < 100ms
# - No crashes or errors

# Monitor during load test
watch -n 5 'curl -s http://node-1:9090/metrics | grep -E "elara_(messages_sent|messages_dropped|message_latency)"'
```

### Log Validation

```bash
# Check for errors in logs
journalctl -u elara-node -n 1000 | grep -i error

# Expected: No critical errors

# Check for warnings
journalctl -u elara-node -n 1000 | grep -i warn

# Expected: No unexpected warnings

# Check startup logs
journalctl -u elara-node -n 100 | grep -i "started\|initialized\|listening"

# Expected:
# - "Node started successfully"
# - "Health server listening on 0.0.0.0:8080"
# - "Metrics server listening on 0.0.0.0:9090"
```

### Monitoring Integration Validation

```bash
# Check Prometheus is scraping metrics
curl http://prometheus:9090/api/v1/targets | jq '.data.activeTargets[] | select(.labels.job=="elara-nodes")'

# Expected: All nodes in "up" state

# Check alerting rules are loaded
curl http://prometheus:9090/api/v1/rules | jq '.data.groups[] | select(.name=="elara_node")'

# Expected: All alert rules present

# Trigger test alert (optional)
# Temporarily set threshold very low to trigger alert
# Verify alert fires in Alertmanager
```

---

## Troubleshooting

### Deployment Failures

#### Issue: Binary fails to start

**Symptoms**:
- Service fails to start
- Immediate exit after start
- "Exec format error" in logs

**Diagnosis**:
```bash
# Check binary architecture
file /usr/local/bin/elara-node

# Expected: ELF 64-bit LSB executable, x86-64

# Check binary permissions
ls -l /usr/local/bin/elara-node

# Expected: -rwxr-xr-x

# Check dependencies
ldd /usr/local/bin/elara-node

# Expected: All dependencies found
```

**Resolution**:
```bash
# Download correct architecture binary
uname -m  # Check system architecture

# For x86_64
wget https://releases.example.com/elara-node-linux-amd64

# For ARM64
wget https://releases.example.com/elara-node-linux-arm64

# Set correct permissions
chmod +x /usr/local/bin/elara-node
```

#### Issue: Configuration validation fails

**Symptoms**:
- "Configuration is invalid" error
- Service fails to start
- TOML parse errors

**Diagnosis**:
```bash
# Validate configuration
elara-node --config /etc/elara/config.toml --validate

# Check TOML syntax
cat /etc/elara/config.toml | toml-lint

# Check for common issues
grep -E '(^\s*$|^#)' /etc/elara/config.toml  # Empty lines and comments
```

**Resolution**:
```bash
# Fix TOML syntax errors
# - Ensure proper quoting of strings
# - Ensure proper array syntax
# - Ensure proper table syntax

# Validate required fields
# - node_id must be set
# - bind_address must be valid
# - peers must be array of strings

# Example valid configuration
cat > /etc/elara/config.toml <<EOF
[node]
node_id = "node-1"
bind_address = "0.0.0.0:7777"
peers = ["node-2:7777", "node-3:7777"]
EOF
```

#### Issue: Port already in use

**Symptoms**:
- "Address already in use" error
- Service fails to start
- Health/metrics endpoints unreachable

**Diagnosis**:
```bash
# Check what's using the ports
ss -tuln | grep -E ':(7777|8080|9090)'

# Check if old process is still running
ps aux | grep elara-node

# Check systemd status
systemctl status elara-node
```

**Resolution**:
```bash
# Kill old process
pkill -9 elara-node

# Or stop via systemd
systemctl stop elara-node

# Wait for ports to be released
sleep 5

# Verify ports are free
ss -tuln | grep -E ':(7777|8080|9090)'

# Start service
systemctl start elara-node
```

### Container Deployment Issues

#### Issue: Container fails health check

**Symptoms**:
- Container restarts repeatedly
- Health check endpoint returns 503
- Kubernetes marks pod as not ready

**Diagnosis**:
```bash
# Check container logs
docker logs elara-node-1

# Or in Kubernetes
kubectl logs -n elara-production elara-node-0

# Check health endpoint
docker exec elara-node-1 curl -v http://localhost:8080/health

# Check if process is running
docker exec elara-node-1 ps aux | grep elara-node
```

**Resolution**:
```bash
# Increase health check timeout
# In Docker Compose:
healthcheck:
  timeout: 10s  # Increase from 5s

# In Kubernetes:
livenessProbe:
  timeoutSeconds: 10  # Increase from 5s

# Increase initial delay
# In Kubernetes:
livenessProbe:
  initialDelaySeconds: 60  # Increase from 30s
```

#### Issue: Kubernetes pods can't communicate

**Symptoms**:
- Pods show 0 active connections
- Connection failures in logs
- Pods are healthy but isolated

**Diagnosis**:
```bash
# Check network policies
kubectl get networkpolicies -n elara-production

# Test pod-to-pod connectivity
kubectl exec -n elara-production elara-node-0 -- nc -u -zv elara-node-1.elara-node.elara-production.svc.cluster.local 7777

# Check service endpoints
kubectl get endpoints -n elara-production elara-node
```

**Resolution**:
```bash
# Create network policy to allow traffic
cat <<EOF | kubectl apply -f -
apiVersion: networking.k8s.io/v1
kind: NetworkPolicy
metadata:
  name: elara-node-allow
  namespace: elara-production
spec:
  podSelector:
    matchLabels:
      app: elara-node
  policyTypes:
  - Ingress
  - Egress
  ingress:
  - from:
    - podSelector:
        matchLabels:
          app: elara-node
    ports:
    - protocol: UDP
      port: 7777
    - protocol: TCP
      port: 8080
    - protocol: TCP
      port: 9090
  egress:
  - to:
    - podSelector:
        matchLabels:
          app: elara-node
    ports:
    - protocol: UDP
      port: 7777
EOF
```

### Performance Issues

#### Issue: High latency after deployment

**Symptoms**:
- P95 latency > 1000ms
- Slow message processing
- User complaints

**Diagnosis**:
```bash
# Check CPU usage
top -bn1 | grep elara-node

# Check memory usage
curl http://localhost:9090/metrics | grep elara_memory_usage_bytes

# Check network latency
ping -c 10 peer-node

# Check time drift
curl http://localhost:9090/metrics | grep elara_time_drift_ms
```

**Resolution**:
```bash
# If CPU saturation: scale horizontally
# Add more nodes to distribute load

# If memory pressure: increase memory limit
# Edit systemd service or Kubernetes resources

# If network latency: check network infrastructure
# Verify MTU, check for packet loss

# If time drift: fix time synchronization
systemctl restart chronyd
```

---

## Best Practices

### Pre-Deployment Checklist

- [ ] Configuration validated
- [ ] Binary/image tested in staging
- [ ] Rollback plan prepared
- [ ] Monitoring alerts configured
- [ ] On-call team notified
- [ ] Maintenance window scheduled (if needed)
- [ ] Backup of current version available
- [ ] Health checks passing in staging
- [ ] Load tests passing in staging
- [ ] Documentation updated

### During Deployment

- [ ] Monitor health endpoints continuously
- [ ] Monitor metrics for anomalies
- [ ] Check logs for errors
- [ ] Verify connectivity between nodes
- [ ] Test with small traffic sample first
- [ ] Have rollback command ready
- [ ] Keep communication channel open with team

### Post-Deployment

- [ ] Verify all nodes are healthy
- [ ] Verify connectivity is established
- [ ] Run smoke tests
- [ ] Monitor for 1 hour minimum
- [ ] Check alerting is working
- [ ] Update runbook if needed
- [ ] Document any issues encountered
- [ ] Notify stakeholders of completion

### Security Best Practices

- [ ] Run as non-root user
- [ ] Use read-only root filesystem
- [ ] Drop unnecessary capabilities
- [ ] Enable SELinux/AppArmor
- [ ] Use secrets management for sensitive data
- [ ] Enable TLS for metrics/health endpoints (if exposed externally)
- [ ] Regularly update dependencies
- [ ] Monitor security advisories

---

## Additional Resources

- [Configuration Guide](CONFIGURATION.md) - Detailed configuration reference
- [Monitoring Guide](MONITORING.md) - Monitoring and alerting setup
- [Operational Runbook](RUNBOOK.md) - Day-to-day operations
- [Performance Guide](../performance/PERFORMANCE_GUIDE.md) - Performance tuning
- [Architecture Documentation](../architecture/COMPREHENSIVE_ARCHITECTURE.md) - System architecture

---

**Document Version**: 1.0  
**Last Updated**: 2024  
**Maintained By**: ELARA Operations Team
