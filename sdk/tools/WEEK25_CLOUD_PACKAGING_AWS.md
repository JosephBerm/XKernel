# Week 25: XKernal Cognitive Substrate - AWS Cloud Packaging Architecture

**Project**: XKernal Cognitive Substrate OS
**Phase**: Phase 3 - Cloud Deployment Infrastructure
**Week**: 25 (Post-Production Hardening)
**Engineer**: L3 SDK Tooling, Packaging & Documentation
**Status**: Design & Implementation Sprint

---

## Executive Summary

Week 25 initiates AWS cloud packaging for the Cognitive Substrate platform. This document specifies the infrastructure-as-code strategy, automated AMI build pipeline, cloud-native telemetry integration, and reproducible benchmark infrastructure. The design targets enterprise-grade reliability (99.95% uptime SLO), cost optimization, and seamless scaling across EC2, RDS, and CloudWatch ecosystems.

---

## 1. Objectives & Deliverables

### Primary Objectives
- Design and implement reproducible AMI build pipeline using Packer
- Develop IaC templates for production CloudFormation and Terraform configurations
- Integrate CloudWatch metrics, logs, and alarms with Substrate runtime
- Establish RDS integration for persistent metadata and telemetry storage
- Create benchmark harness for reproducible performance validation
- Generate cost estimation models and scaling recommendations

### Deliverables Timeline
- **Week 25.1**: AMI design, Packer template, base image validation
- **Week 25.2**: CloudFormation/Terraform stacks, deployment automation
- **Week 25.3**: CloudWatch/RDS integration, metric pipeline
- **Week 25.4**: Benchmark harness, cost analysis, documentation

---

## 2. AMI Build Pipeline (Packer)

### 2.1 Architecture Design

```
Packer Configuration → EC2 Build Instance → System Hardening → Substrate Runtime
    ↓                        ↓                      ↓                ↓
packer.hcl          ubuntu-22.04-lts      Security Patching    Application Build
   Config            source (AWS)          + Dependencies       + Libraries
```

### 2.2 Packer Template Specification

**File**: `/infrastructure/packer/xkernal-substrate.hcl`

```hcl
source "amazon-ebs" "substrate" {
  ami_name        = "xkernal-substrate-{{timestamp}}"
  instance_type   = "t4g.xlarge"  # Graviton3 for cost efficiency
  region          = "us-east-1"
  root_volume_size = 100
  volume_type     = "gp3"

  tags = {
    Name        = "XKernal-Substrate"
    Phase       = "3-Production"
    BuildDate   = "{{timestamp}}"
    Version     = "1.0.0"
  }
}

build {
  sources = ["source.amazon-ebs.substrate"]

  provisioner "file" {
    source      = "../sdk/release/"
    destination = "/opt/xkernal"
  }

  provisioner "shell" {
    scripts = [
      "${path.root}/scripts/base-system.sh",
      "${path.root}/scripts/substrate-runtime.sh",
      "${path.root}/scripts/cloudwatch-agent.sh",
      "${path.root}/scripts/security-hardening.sh"
    ]
  }
}
```

### 2.3 Build Validation

- **Image Scanning**: Trivy vulnerability scanning pre-release
- **Boot Testing**: Verify EC2 launch in 45s, runtime readiness in 90s
- **Configuration Audit**: Confirm all tuning parameters, security groups
- **Telemetry Verification**: Validate CloudWatch agent connectivity

---

## 3. Infrastructure-as-Code (IaC) Strategy

### 3.1 Dual-Stack Approach

**Terraform** (Primary)
- Modular, version-controlled infrastructure
- State management with S3 + DynamoDB locking
- Environment-specific tfvars (dev, staging, prod)
- Automated plan/apply CI/CD pipeline

**CloudFormation** (Secondary)
- AWS-native template for enterprise governance
- Stack change sets for safe deployments
- Integration with AWS Service Catalog
- Role-based access control (RBAC) alignment

### 3.2 Terraform Module Structure

```
terraform/
├── modules/
│   ├── vpc/
│   │   ├── main.tf (VPC, subnets, NAT, IGW)
│   │   ├── variables.tf
│   │   └── outputs.tf
│   ├── compute/
│   │   ├── main.tf (ASG, launch template, security groups)
│   │   ├── variables.tf
│   │   └── outputs.tf
│   ├── database/
│   │   ├── main.tf (RDS Aurora PostgreSQL, parameter groups)
│   │   ├── variables.tf
│   │   └── outputs.tf
│   └── monitoring/
│       ├── main.tf (CloudWatch dashboards, alarms, log groups)
│       ├── variables.tf
│       └── outputs.tf
├── environments/
│   ├── dev/
│   │   ├── terraform.tfvars
│   │   └── main.tf
│   ├── staging/
│   ├── prod/
│   └── backend.tf
```

### 3.3 Key Infrastructure Components

**VPC Configuration**
- 3 AZ deployment for HA
- Public subnets: NAT gateways, ALB
- Private subnets: EC2 instances, RDS
- Endpoint services: S3, CloudWatch Logs, ECR

**Auto Scaling Group**
- Min: 2, Desired: 4, Max: 16 instances
- Mixed instance types: t4g.xlarge (70%), c7g.xlarge (30%)
- Target tracking: 65% CPU, 70% memory
- Termination policy: OldestInstance, Oldest Launch Template

**Security Groups**
- ALB: Ingress 80/443 from 0.0.0.0/0
- EC2: Ingress 9090 (Prometheus), 5432 (RDS tunnel) from internal
- RDS: Ingress 5432 from EC2 security group only

---

## 4. CloudWatch Integration

### 4.1 Metrics Pipeline

**Substrate Runtime Instrumentation**
```rust
// src/telemetry/cloudwatch.rs
pub struct CloudWatchMetrics {
    client: CloudWatchClient,
    namespace: String,
    buffer: MetricBuffer,
    flush_interval_ms: u64,
}

impl CloudWatchMetrics {
    pub async fn record_substrate_latency(&self, latency_ms: f64, operation: &str) {
        self.buffer.push(MetricDatum {
            metric_name: "SubstrateLatencyMs".to_string(),
            dimensions: vec![Dimension {
                name: "Operation".to_string(),
                value: operation.to_string(),
            }],
            value: Some(latency_ms),
            timestamp: SystemTime::now(),
        });
    }
}
```

**Key Metrics**
- `SubstrateLatencyMs` (p50, p99, p99.9 percentiles)
- `CognitiveOpsThroughput` (ops/sec)
- `MemoryUtilizationPercent` (heap, RSS, cached)
- `RDSConnectionPoolSize` (active, idle, waiting)
- `ErrorRate` (fatal, recoverable, timeout)

### 4.2 Log Aggregation

- **Log Groups**: `/xkernal/substrate/{environment}/{instance-id}`
- **Retention**: 30 days production, 7 days dev
- **Structured Logging**: JSON format with trace IDs, request UUIDs
- **Log Insights Queries**: Pre-built for latency analysis, error breakdown

### 4.3 Alarms & Notifications

```hcl
resource "aws_cloudwatch_metric_alarm" "substrate_latency_p99" {
  alarm_name          = "substrate-latency-p99-prod"
  comparison_operator = "GreaterThanThreshold"
  evaluation_periods  = 2
  metric_name         = "SubstrateLatencyMs"
  namespace           = "XKernal/Substrate"
  period              = 300
  statistic           = "p99"
  threshold           = 500
  alarm_actions       = [aws_sns_topic.oncall.arn]
  treat_missing_data  = "notBreaching"
}
```

---

## 5. RDS Integration

### 5.1 Database Design

**Aurora PostgreSQL 15**
- Multi-AZ deployment (2 replicas, 1 writer)
- Instance class: db.r6g.2xlarge (Graviton2, 8 vCPU, 64 GB RAM)
- Storage: 500 GB gp3, auto-scaling to 1000 GB
- Backup: 35-day retention, automated snapshots at 2 AM UTC

**Schema: Substrate Telemetry**
```sql
CREATE TABLE substrate_events (
    event_id BIGINT PRIMARY KEY,
    timestamp TIMESTAMPTZ NOT NULL,
    instance_id VARCHAR(32) NOT NULL,
    operation_type VARCHAR(64) NOT NULL,
    latency_ms FLOAT8 NOT NULL,
    status VARCHAR(16) NOT NULL,
    error_code VARCHAR(16),
    INDEX idx_timestamp (timestamp DESC),
    INDEX idx_instance (instance_id, timestamp DESC)
);

CREATE TABLE rds_connection_metrics (
    id BIGINT PRIMARY KEY,
    timestamp TIMESTAMPTZ NOT NULL,
    pool_size INT,
    active_connections INT,
    idle_connections INT
);
```

### 5.2 Connection Pooling

**PgBouncer Proxy** (t3.medium on EC2)
- Pool mode: transaction
- Max client conn: 1000
- Min pool size: 10, Res pool size: 5
- Statement cache: 16MB per client
- Timeout: 30s idle, 60s login

### 5.3 High Availability

- **Failover**: RDS automated failover (< 60s)
- **Read Replicas**: 2 cross-region for DR (eu-west-1, ap-northeast-1)
- **Backup Strategy**: Daily snapshots to S3, encrypted at rest (KMS)
- **Encryption**: TLS 1.3 in-transit, AES-256 at-rest

---

## 6. Benchmark Harness Design

### 6.1 Structure

**Directory**: `/benches/`

```
benches/
├── harness/
│   ├── main.rs (benchmark orchestrator)
│   ├── workload.rs (configurable load patterns)
│   ├── validator.rs (result verification)
│   └── reporter.rs (output formats: JSON, HTML, CloudWatch)
├── workloads/
│   ├── cognitive_op_baseline.yaml
│   ├── memory_stress_test.yaml
│   └── concurrent_scale_test.yaml
└── scripts/
    ├── deploy-benchmark.sh
    ├── run-benchmarks.sh
    └── analyze-results.sh
```

### 6.2 Benchmark Scenarios

1. **Baseline Cognitive Operations**
   - 1000 ops, 16 concurrent workers
   - Metrics: p50/p99/p99.9 latency, throughput
   - Expected: <100ms p99, >10k ops/sec

2. **Memory Scaling**
   - Allocate 1GB → 16GB in 1GB increments
   - Measure heap fragmentation, GC pause times
   - Expected: linear scaling, <50ms max GC pause

3. **Concurrent Scale Test**
   - 1 → 512 concurrent clients
   - Monitor connection pool, RDS latency
   - Expected: linear throughput, <1% error rate

### 6.3 Benchmark Execution (Cloud)

```bash
# Deploy benchmark infrastructure
terraform apply -target=module.benchmark_stack -var-file=prod.tfvars

# Run benchmarks on 4-instance cluster (1h duration)
./scripts/run-benchmarks.sh \
  --environment prod \
  --duration 3600 \
  --concurrency 128 \
  --output-bucket s3://xkernal-benchmark-results/

# Generate comparison report
./scripts/analyze-results.sh \
  --baseline-run 2025-02-01 \
  --current-run $(date +%Y-%m-%d) \
  --html-output /tmp/benchmark-report.html
```

---

## 7. Cost Estimation & Optimization

### 7.1 Monthly Cost Breakdown (Production)

| Component | Quantity | Instance Type | Cost/Month |
|-----------|----------|---------------|-----------|
| EC2 Compute (ASG) | 4-16 | t4g.xlarge | $2,400 |
| RDS Database | 1 | db.r6g.2xlarge | $3,800 |
| RDS Storage | 500GB-1TB | gp3 | $450 |
| NAT Gateway | 3 AZ | - | $1,350 |
| CloudWatch Logs | 50GB/month | - | $600 |
| Data Transfer (egress) | 100GB | - | $900 |
| **Total (Baseline)** | - | - | **$9,500** |

### 7.2 Optimization Strategies

- **Reserved Instances**: 1-year RDS reservation (-40%), ASG spot instances (-70% peak)
- **Graviton Migration**: 20% cost reduction vs x86
- **CloudWatch Logs**: VPC Flow Logs to S3 bucket (-80% logging cost)
- **Auto-Scaling**: Off-peak instance reduction (10 PM - 6 AM), save $1,200/month

---

## 8. Deployment & CI/CD

### 8.1 Automated Pipeline

```yaml
# .github/workflows/deploy-aws.yml
stages:
  - validate: terraform validate, Packer build
  - plan: terraform plan, cost estimation
  - review: manual approval
  - deploy: terraform apply, AMI rollout, health checks
  - benchmark: run validation benchmarks, CloudWatch metrics
```

### 8.2 Rollback Strategy

- Blue/Green deployment: ALB traffic split 10%/90% for 1 hour
- Automatic rollback if error rate > 1%, p99 latency > 500ms
- EBS snapshots retained for 30-day point-in-time recovery

---

## 9. Security & Compliance

- **Encryption**: TLS 1.3 (transit), KMS AES-256 (at-rest)
- **IAM Policies**: Least-privilege, role-based access, no root key usage
- **Network**: VPC isolation, security group segmentation, NACLs
- **Scanning**: Trivy/Grype container scanning, AWS Config compliance
- **Audit Logging**: CloudTrail all API calls, S3 MFA delete enabled

---

## 10. Success Criteria & Milestones

| Milestone | Target Date | Success Criteria |
|-----------|-------------|------------------|
| AMI Build Pipeline | Week 25.1 | <5min build time, 0 vulnerabilities |
| IaC Automation | Week 25.2 | Full prod environment via terraform apply |
| CloudWatch Integration | Week 25.3 | All metrics flowing, <1s dashboard update |
| Benchmark Harness | Week 25.4 | Automated weekly runs, cost tracking enabled |

---

## References & Appendix

- AWS Well-Architected Framework (Reliability, Performance, Cost Optimization)
- Terraform AWS Provider v5.x documentation
- Packer EC2 builder best practices
- PostgreSQL 15 performance tuning guide
- Phase 2 & 3 SLO definitions (99.95% uptime, <200ms p99 latency)
