# Engineer 10 — SDK: Tooling, Packaging & Documentation — Week 25

## Phase: 3 (Cloud Deployment, Documentation & Launch)

## Weekly Objective
Begin cloud packaging for AWS. Design and implement AMI (Amazon Machine Image) for Cognitive Substrate deployment. Create infrastructure-as-code for cloud deployment. Design cloud-specific tooling integration.

## Document References
- **Primary:** Section 6.4 — Phase 3, Week 25-26 (Cloud packaging), Section 6.4 — Week 26-30 (Complete cloud deployment)
- **Supporting:** Section 3.5.3 (cs-pkg registry for cloud packages)

## Deliverables
- [ ] AWS AMI specification and build configuration
- [ ] CloudFormation templates for Cognitive Substrate deployment
- [ ] VPC, security groups, and networking configuration
- [ ] Cloud-specific cs-pkg packages (AWS tools, monitoring integration)
- [ ] CloudWatch integration for cs-top and other monitoring
- [ ] RDS database setup for cs-pkg registry (optional cloud variant)
- [ ] Documentation: AWS deployment guide
- [ ] Terraform configuration as alternative to CloudFormation
- [ ] Reproducible benchmark harness in `/benches/` directory (Addendum v2.5.1 — Correction 2: Benchmark Methodology)

## Technical Specifications
### AWS AMI Content
```
Cognitive Substrate AMI (cs-runtime-linux-x86_64-v1.0.0)

Base: Amazon Linux 2 (minimal)
├── Kernel: Linux 6.x (Cognitive Substrate enabled)
├── Runtime: Cognitive Substrate runtime
├── SDK: cs-sdk, cs-pkg, cs-ctl
├── Tools: cs-trace, cs-replay, cs-profile, cs-capgraph, cs-top
├── Monitoring: CloudWatch agent, prometheus-node-exporter
├── Logging: CloudWatch Logs agent
└── Configuration: /etc/cs-runtime/cs-config.toml
```

### CloudFormation Template Structure
```yaml
AWSTemplateFormatVersion: '2010-09-09'
Description: Cognitive Substrate Deployment

Parameters:
  ImageId:
    Type: String
    Description: AMI ID for Cognitive Substrate
  InstanceType:
    Type: String
    Default: t3.xlarge
    Description: EC2 instance type

Resources:
  CognitiveSubstrateVPC:
    Type: AWS::EC2::VPC
    Properties:
      CidrBlock: 10.0.0.0/16
      EnableDnsHostnames: true

  CognitiveSubstrateInstance:
    Type: AWS::EC2::Instance
    Properties:
      ImageId: !Ref ImageId
      InstanceType: !Ref InstanceType
      SecurityGroupIds: [!Ref CognitiveSubstrateSecurityGroup]
      IamInstanceProfile: !Ref CognitiveSubstrateInstanceProfile

  CognitiveSubstrateSecurityGroup:
    Type: AWS::EC2::SecurityGroup
    Properties:
      GroupDescription: Cognitive Substrate security group
      SecurityGroupIngress:
        - IpProtocol: tcp
          FromPort: 22
          ToPort: 22
          CidrIp: 0.0.0.0/0  # SSH (restrict in production)
        - IpProtocol: tcp
          FromPort: 443
          ToPort: 443
          CidrIp: 0.0.0.0/0  # HTTPS

Outputs:
  InstanceId:
    Value: !Ref CognitiveSubstrateInstance
  PublicIP:
    Value: !GetAtt CognitiveSubstrateInstance.PublicIp
```

### Cloud-Specific cs-pkg Packages
1. **aws-monitoring-integration**: CloudWatch metrics for cs-top
2. **aws-secrets-manager**: Integrate with AWS Secrets Manager
3. **aws-cost-analyzer**: Enhanced cost reporting with AWS cost data
4. **aws-lambda-adapter**: Deploy CTs as Lambda functions

### Reproducible Benchmark Harness (`/benches/`)

The benchmark harness must deliver:

**Workload Generation:**
- Generates all 4 reference workloads deterministically from seed
- Seed-based generation ensures reproducibility across runs
- Workloads: Enterprise Research Team, Code Review, Customer Support, Scientific Discovery

**Deployment Capability:**
- Deploys Cognitive Substrate configuration
- Deploys Linux+Docker baseline configuration
  - Ubuntu 24.04 LTS
  - Docker 27.x
  - Same GPU hardware for fair comparison
  - LangChain 0.3.x / SK 1.x

**Measurement Suite:**
- Runs all 8 measurement dimensions per workload
- Statistical validation: 100 runs minimum, 95% confidence interval, 10 warmup runs discarded
- Collects percentile data: p50, p95, p99, p99.9 for latency metrics
- Measures IPC latency, cold start, and fault recovery times

**Output Formats:**
- Markdown reports with comparison tables
- JSON output for programmatic analysis
- Automated comparison: Cognitive Substrate vs Linux+Docker baseline

## Dependencies
- **Blocked by:** Week 24 Phase 2 completion, cs-pkg registry stable
- **Blocking:** Week 26 Azure/GCP, Week 27-30 documentation portal

## Acceptance Criteria
- [ ] AMI builds successfully and boots in <2 minutes
- [ ] CloudFormation template deploys without manual intervention
- [ ] All debugging tools functional in cloud environment
- [ ] CloudWatch integration shows cs-top metrics
- [ ] cs-pkg registry accessible from cloud instance
- [ ] Terraform configuration produces identical infrastructure to CloudFormation
- [ ] AWS deployment guide sufficient for operators

## Design Principles Alignment
- **Cognitive-Native:** Cloud deployment preserves isolation and capability model
- **Isolation by Default:** Security groups and VPC enforce network isolation
- **Cost Transparency:** CloudWatch integration enables cloud cost tracking
- **Infrastructure as Code:** Terraform/CloudFormation enable reproducible deployments
