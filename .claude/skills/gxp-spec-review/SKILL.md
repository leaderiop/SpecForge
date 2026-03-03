---
name: gxp-spec-review
description: "Review specification documents (URS, FS, DS, CS, Test Specs) for GxP regulatory compliance against FDA 21 CFR Part 11, EU GMP Annex 11, GAMP 5, ICH Q9, and ALCOA+ data integrity principles. Use when reviewing, auditing, or writing markdown specification files such as URS.md, FS.md, DS.md, CS.md, IQ.md, OQ.md, PQ.md, or any requirement document in a specs/ or specifications/ directory. Use when working on files that contain requirement IDs (e.g., URS-001, FS-001, DS-001), traceability matrices, GAMP software category references, ALCOA+ checklists, or regulatory compliance mappings. Use when a user asks to review a spec for compliance, perform a gap analysis against regulations, check traceability between specification levels, validate test specifications against requirements, assess specification completeness, or audit requirement documents before approval. Use when creating or editing specification templates for regulated pharmaceutical, biotech, medical device, clinical trial, or laboratory systems. Use when evaluating audit trail requirements, electronic signature requirements, access control specifications, data integrity controls, or error handling in GxP-critical systems."
---

# GxP Specification Review

This skill reviews specification documents in the repository against GxP regulatory requirements. It evaluates User Requirements Specifications (URS), Functional Specifications (FS), Design Specifications (DS), Configuration Specifications (CS), and Test Specifications (IQ/OQ/PQ) for compliance with FDA 21 CFR Part 11, EU GMP Annex 11, GAMP 5, ICH Q9, and ALCOA+ data integrity principles.

## When to use this skill:

- Reviewing or editing markdown specification files (URS.md, FS.md, DS.md, CS.md, IQ.md, OQ.md, PQ.md) for GxP compliance
- Auditing requirement documents before approval or submission to quality assurance
- Checking forward and backward traceability between specification levels (URS -> FS -> DS -> CS -> IQ/OQ/PQ)
- Validating test specifications (IQ, OQ, PQ) against their corresponding requirement and design documents
- Assessing specification completeness for regulated pharmaceutical, biotech, medical device, or laboratory systems
- Performing gap analysis against FDA 21 CFR Part 11, EU GMP Annex 11, GAMP 5, or ICH Q9 requirements
- Evaluating ALCOA+ data integrity compliance in specification documents
- Reviewing audit trail requirements, electronic signature requirements, or access control specifications
- Creating or editing specification templates for GxP-regulated systems
- Writing or reviewing traceability matrices that map requirements to design and test documents
- Assessing risk classifications and GAMP 5 software category assignments
- Reviewing error handling, disaster recovery, and data retention specifications in regulated contexts
- Preparing specification documents for regulatory audits or inspections
- Working on files in specs/, specifications/, docs/specs/, or similar directories that contain regulatory requirement documents

## Regulatory Knowledge Base

### Primary Regulations

- **FDA 21 CFR Part 11** - Electronic Records; Electronic Signatures
- **EU GMP Annex 11** - Computerised Systems
- **GAMP 5 (2nd Edition, ISPE)** - Good Automated Manufacturing Practice
- **FDA General Principles of Software Validation**
- **ICH Q9** - Quality Risk Management
- **ICH Q10** - Pharmaceutical Quality System

### Data Integrity Guidance

- **WHO TRS 1033 Annex 4** - Data Integrity Guidelines
- **MHRA GxP Data Integrity Guidance** (March 2018)
- **PIC/S PI 041** - Good Practices for Data Management and Integrity
- **FDA Data Integrity and Compliance with Drug CGMP** (December 2018)

### ALCOA+ Principles

All data and documentation in GxP systems must be:

- **Attributable** - Who performed the action and when
- **Legible** - Readable, permanent, preserved in original form
- **Contemporaneous** - Recorded at the time of the activity
- **Original** - First capture preserved or validated true copy
- **Accurate** - Error-free, truthful, edits tracked with original value preserved
- **Complete** - All data present including failures and deviations
- **Consistent** - Sequence of events evident through timestamps
- **Enduring** - Recorded on approved, durable media surviving retention period
- **Available** - Accessible and retrievable for review and audit

---

## GAMP 5 V-Model and Specification Hierarchy

```
Left Side (Specifications)          Right Side (Verification)
------------------------            ------------------------
User Requirements (URS)      <-->   Performance Qualification (PQ)
         |                                      ^
Functional Specs (FS)        <-->   Operational Qualification (OQ)
         |                                      ^
Design Specs (DS)            <-->   Installation Qualification (IQ)
         |                                      ^
Configuration Specs (CS)     <-->   Configuration Verification
         |                                      ^
    Build/Configure                      Execute Tests
```

### Specification Relationships

- **URS drives everything** - All downstream specs trace back to user requirements
- **FS elaborates URS** - Functional specs detail HOW user requirements will be met
- **DS implements FS** - Design specs describe the technical architecture
- **CS parameterizes DS** - Configuration specs document system settings
- **Test specs verify specs** - Each test level verifies its corresponding specification level

### GAMP 5 Software Categories

| Category | Type | Validation Burden | Specification Depth |
|----------|------|-------------------|---------------------|
| **1** | Infrastructure Software | Lowest | Record version, verify installation |
| **3** | Non-Configured Products (COTS as-is) | Low | Verify meets requirements |
| **4** | Configured Products (COTS + config) | Medium | Document and verify configuration |
| **5** | Custom Applications (bespoke) | Highest | Full lifecycle, all specification levels |

---

## Review Methodology

### Severity Definitions

- **Critical** - Direct violation of a regulatory requirement that could lead to data integrity breach, patient safety risk, or regulatory action (Warning Letter, Form 483). Must be fixed before approval.
- **Major** - Significant gap that undermines compliance posture and would likely be cited in an audit. Should be fixed before approval.
- **Minor** - Deviation from best practice that could become a major issue. Should be addressed in next revision.
- **Observation** - Improvement opportunity that strengthens compliance posture but is not a regulatory gap.

### Finding Format

For each finding, provide:

1. **Severity**: Critical / Major / Minor / Observation
2. **Section**: Which specification section is affected
3. **Regulation**: Specific regulatory clause (e.g., "21 CFR 11.10(e)")
4. **Finding**: Clear description of the compliance gap
5. **Evidence**: Quote or reference from specification
6. **Risk**: What could go wrong if not addressed
7. **Recommendation**: Specific remediation with example language

### Severity Escalation Factors

Increase severity when:

- Affects multiple systems or processes
- No compensating controls available
- Historical compliance issues in the area
- High-visibility regulatory area
- Repeat finding from previous review

### Severity Mitigation Factors

May decrease severity when:

- Strong compensating controls exist
- Limited scope or impact
- Procedural workaround available
- Low regulatory focus area
- First occurrence

---

## Specification Review Domains

### 1. User Requirements Specification (URS)

#### Required Sections

| Section | Required Content | Regulatory Driver |
|---------|-----------------|-------------------|
| Introduction | Purpose, scope, system overview, abbreviations | GAMP 5 |
| Regulatory Requirements | Applicable regulations (21 CFR Part 11, EU Annex 11, etc.) | FDA/EMA |
| User Groups | Roles, responsibilities, access levels | 21 CFR 11.10(g) |
| Business Process | Process flow diagrams, use cases, workflows | GAMP 5 |
| Functional Requirements | What the system must do, organized by process | GAMP 5 |
| Data Requirements | Data types, retention, integrity, ALCOA+ | 21 CFR 11.10(c) |
| Interface Requirements | System integrations, data exchange | EU Annex 11.5 |
| Performance Requirements | Response times, capacity, availability | GAMP 5 |
| Security Requirements | Authentication, authorization, encryption | 21 CFR 11.10(d) |
| Audit Trail Requirements | What must be audited, retention | 21 CFR 11.10(e) |
| Electronic Signature Requirements | Where required, signature manifestation | 21 CFR 11.50 |
| Reporting Requirements | Reports, formats, distribution | GxP operations |
| Compliance Requirements | Specific regulatory requirements | 21 CFR Part 11 |
| Training Requirements | User training needs | EU Annex 11.2 |
| Constraints | Technical, operational, regulatory constraints | GAMP 5 |
| Assumptions | Dependencies and assumptions | GAMP 5 |
| Risk Assessment | Initial risk assessment per ICH Q9 | ICH Q9 |

#### Quality Criteria

**Good URS characteristics:**

- Written from the user's perspective (what, not how)
- Business language, not technical jargon
- Each requirement is atomic (one testable statement)
- Unique identifiers (URS-XXX-NNN format)
- Clear acceptance criteria per requirement
- Prioritized (Critical/Major/Minor or MoSCoW)
- Both functional and non-functional requirements
- Risk-based detail level per ICH Q9

**Bad URS characteristics (flag as findings):**

- Solution-oriented ("shall use Oracle database")
- Ambiguous language ("fast", "user-friendly", "efficient", "approximately")
- Compound requirements with AND/OR making testing difficult
- Missing regulatory requirements
- No data integrity requirements
- Untestable statements ("shall comply with all regulations")
- Copy-pasted from vendor documentation without adaptation

#### Common Deficiencies

1. Missing data integrity requirements (no ALCOA+ principles)
2. Ambiguous language ("approximately", "adequate", "as needed")
3. Solution-oriented instead of need-oriented
4. No risk assessment (all requirements treated equally)
5. Missing audit trail requirements
6. Incomplete user roles (no segregation of duties)
7. No performance criteria with specific metrics
8. Missing disaster recovery requirements
9. No data retention requirements
10. Untestable requirements

#### ALCOA+ Application to URS

| Principle | How It Applies |
|-----------|---------------|
| Attributable | Author, reviewer, approver clearly identified with signatures |
| Legible | Clear formatting, defined terms, no ambiguous abbreviations |
| Contemporaneous | Date of creation, revision history, change reasons documented |
| Original | Master document location identified, version controlled |
| Accurate | Reviewed for correctness, no contradictions |
| Complete | All GxP processes covered, no TBDs in approved version |
| Consistent | Terminology consistent throughout, no conflicting requirements |
| Enduring | Stored in validated document management system |
| Available | Accessible to all stakeholders, retention period defined |

---

### 2. Functional Specification (FS)

#### Required Sections

| Section | Required Content | Regulatory Driver |
|---------|-----------------|-------------------|
| Traceability Matrix | URS ID to FS ID mapping | GAMP 5 |
| System Overview | Functional architecture, modules | GAMP 5 |
| User Interface Specs | Screen designs, navigation, field definitions | Usability |
| Business Process Details | Step-by-step process flows, decision logic | GAMP 5 |
| Business Rules | Calculations, validations, constraints | Data Integrity |
| Data Processing | Input validation, transformation, output | ALCOA+ |
| System Interfaces | API specs, data formats, protocols | EU Annex 11.5 |
| Reports Specification | Layout, calculations, parameters | GxP Operations |
| Audit Trail Behavior | What triggers audit entries, format | 21 CFR 11.10(e) |
| Electronic Signature Behavior | Signature workflows, manifestation | 21 CFR 11.50 |
| Security Functions | Login, password rules, timeout | 21 CFR 11.10(d) |
| Error Handling | Error messages, recovery procedures | EU Annex 11.13 |
| Data Migration | Migration rules if applicable | Data Integrity |
| Archival Functions | Archive/restore procedures | 21 CFR 11.10(c) |

#### Quality Criteria

**Good FS characteristics:**

- Clear mapping to each URS requirement
- Describes system behavior in detail
- Includes normal AND exception scenarios
- Screen mockups/wireframes for user interfaces
- Business rules clearly defined with formulas
- Data validation rules specified (format, range, mandatory)
- Error handling defined per scenario
- Workflow diagrams with decision points

**Bad FS characteristics (flag as findings):**

- Copy of URS without elaboration
- Contains technical implementation details (belongs in DS)
- Missing error/exception scenarios
- No user interface descriptions
- Undefined business rules
- Missing data transformation logic

#### Common Deficiencies

1. Missing exception handling (only happy path)
2. No UI specifications ("user enters data" without field details)
3. Incomplete business rules (calculations not fully defined)
4. Missing audit trail triggers
5. No data validation rules
6. Unclear workflows (decision points undocumented)
7. Missing interface error handling
8. No performance specifications with concrete targets
9. Incomplete security functions
10. Missing report specifications

#### ALCOA+ Application to FS

The FS must specify HOW the system will maintain ALCOA+:

- How attribution is captured (user context in every operation)
- How legibility is ensured (formatting, display rules)
- How contemporaneous recording is enforced (server timestamps)
- How original data is preserved during edits (audit trail)
- How accuracy is validated (business rules, validation logic)
- How completeness is enforced (mandatory fields, sequence checks)
- How consistency is maintained (referential integrity, transaction boundaries)
- How enduring storage is achieved (backup, archival)
- How availability is guaranteed (query performance, retrieval)

---

### 3. Design Specification (DS)

#### Required Sections

| Section | Required Content | Regulatory Driver |
|---------|-----------------|-------------------|
| Traceability Matrix | FS ID to DS ID mapping | GAMP 5 |
| System Architecture | Components, layers, deployment | GAMP 5 |
| Technology Stack | Languages, frameworks, libraries, versions | Change Control |
| Database Design | Schema, constraints, indexes, triggers | Data Integrity |
| API Design | Endpoints, methods, payloads, authentication | EU Annex 11.5 |
| Security Architecture | Encryption, authentication, authorization | 21 CFR 11.10(d) |
| Audit Trail Implementation | Tables, triggers, immutability approach | 21 CFR 11.10(e) |
| Electronic Signature Impl. | Signature storage, verification | 21 CFR 11.50 |
| Data Flow Diagrams | How data moves through system | ALCOA+ |
| Integration Design | Middleware, queues, protocols | EU Annex 11.5 |
| Error Handling Design | Logging, alerting, recovery | EU Annex 11.13 |
| Performance Design | Caching, indexing, optimization | Availability |
| Backup/Recovery Design | Strategy, RTO/RPO | EU Annex 11.17 |
| Infrastructure Design | Servers, network, storage | GAMP 5 |
| Development Standards | Coding standards, review process | GAMP 5 |

#### Quality Criteria

**Good DS characteristics:**

- Clear technical architecture with diagrams
- Database schema with all constraints defined
- API contracts with request/response schemas
- Security implementation details (not "security through obscurity")
- Technology stack justified with rationale
- Scalability approach defined
- Error handling implementation with recovery paths
- Performance optimization approach with benchmarks

**Bad DS characteristics (flag as findings):**

- Generic architecture diagrams without specifics
- Missing database constraints (everything nullable, no FKs)
- No API versioning strategy
- No disaster recovery design
- Missing concurrency handling
- Mutable audit trail design

#### Common Deficiencies

1. Missing database constraints (allows data integrity violations)
2. No audit trail triggers (audit trail can be bypassed)
3. Weak authentication design (shared accounts possible)
4. No version control strategy for code/config
5. Missing encryption design (data at rest/in transit)
6. No concurrency control (race conditions possible)
7. Missing error recovery (system state after failures undefined)
8. No performance benchmarks
9. Incomplete API specifications
10. No disaster recovery design (RPO/RTO not defined)

#### ALCOA+ Application to DS

Design must enforce ALCOA+ at the technical level:

- Database triggers for attribution (Attributable)
- Character encoding for legibility (Legible)
- NTP time synchronization design (Contemporaneous)
- Immutable audit storage, append-only (Original)
- Referential integrity constraints (Accurate)
- NOT NULL constraints on required fields (Complete)
- Transaction boundaries, ACID properties (Consistent)
- Backup/archive design with verified restore (Enduring)
- Query performance and indexing for retrieval (Available)

---

### 4. Configuration Specification (CS)

#### Required Sections

| Section | Required Content | Regulatory Driver |
|---------|-----------------|-------------------|
| Traceability Matrix | DS ID to CS ID mapping | GAMP 5 |
| System Parameters | All configurable settings, defaults, rationale | Control |
| Security Configuration | Password policy, timeout, encryption settings | 21 CFR 11.10(d) |
| Audit Configuration | Audit levels, retention, purging rules | 21 CFR 11.10(e) |
| User Roles/Permissions | Role definitions, permission matrix | 21 CFR 11.10(g) |
| Interface Configuration | Connection strings, timeouts, retries | EU Annex 11.5 |
| Report Configuration | Parameters, distribution lists | Operations |
| Workflow Configuration | Approval chains, escalations | Business Process |
| Master Data | Reference data, lookup values | Data Integrity |
| Backup Configuration | Schedule, retention, location | EU Annex 11.17 |
| Performance Tuning | Cache sizes, thread pools, timeouts | Performance |
| Environment Settings | Dev/Test/Prod differences | Change Control |

#### Quality Criteria

**Good CS characteristics:**

- All configurable parameters documented with defaults
- Rationale for non-default values
- Configuration change process defined
- Environment-specific settings separated
- Validation rules for each parameter
- Dependencies between settings noted

**Bad CS characteristics (flag as findings):**

- Hard-coded values in specification (non-configurable critical settings)
- Default passwords documented
- Missing security configurations
- No validation boundaries for parameters
- No change control process for configuration changes

#### Common Deficiencies

1. Missing security settings (default passwords, no timeout)
2. No validation rules (invalid configurations possible)
3. Hardcoded critical values (non-configurable)
4. Missing audit configuration (retention not defined)
5. No role definitions (permissions unclear)
6. Missing environment separation
7. No change control process
8. Missing parameter dependencies
9. No backup configuration
10. Missing interface timeouts

---

### 5. Test Specification (IQ/OQ/PQ)

#### Installation Qualification (IQ) Required Content

| Section | Required Content | Regulatory Driver |
|---------|-----------------|-------------------|
| Hardware Verification | Server specs, network, storage | GAMP 5 |
| Software Verification | OS, database, middleware versions | GAMP 5 |
| Component Installation | Application components, services | GAMP 5 |
| Security Verification | Certificates, encryption, ports | 21 CFR 11.10(d) |
| Connectivity Tests | Network, database, interfaces | EU Annex 11.5 |
| User Access Verification | Accounts created, permissions set | 21 CFR 11.10(g) |
| Backup Verification | Backup systems operational | EU Annex 11.17 |
| Documentation Check | Manuals, SOPs available | GAMP 5 |

#### Operational Qualification (OQ) Required Content

| Section | Required Content | Regulatory Driver |
|---------|-----------------|-------------------|
| Functional Tests | Each FS requirement tested | GAMP 5 |
| Security Tests | Authentication, authorization, encryption | 21 CFR 11.10(d) |
| Audit Trail Tests | All CRUD operations generate audit entries | 21 CFR 11.10(e) |
| E-Signature Tests | Signature workflows, manifestation | 21 CFR 11.50 |
| Interface Tests | Data exchange, error handling | EU Annex 11.5 |
| Report Tests | All reports, calculations correct | Data Integrity |
| Boundary Tests | Min/max values, edge cases | Robustness |
| Negative Tests | Invalid inputs, error conditions | EU Annex 11.13 |
| Concurrency Tests | Multi-user scenarios | Data Integrity |

#### Performance Qualification (PQ) Required Content

| Section | Required Content | Regulatory Driver |
|---------|-----------------|-------------------|
| Business Process Tests | End-to-end workflows with real data | GAMP 5 |
| User Acceptance Tests | Real users, production scenarios | Fitness for Use |
| Performance Tests | Load, stress, volume testing | Availability |
| Disaster Recovery Tests | Backup/restore procedures | EU Annex 11.17 |
| Data Migration Tests | Data conversion if applicable | Data Integrity |
| Integration Tests | Full ecosystem testing | EU Annex 11.5 |
| Compliance Tests | Regulatory requirements met | 21 CFR Part 11 |
| Training Verification | Users adequately trained | EU Annex 11.2 |

#### Quality Criteria

**Good test specification characteristics:**

- Clear traceability to requirements/design
- Unambiguous test steps (no interpretation needed)
- Explicit expected results (objective pass/fail)
- Defined test data (specific values, not "valid data")
- Prerequisites clearly stated
- Risk-based test coverage with justification
- Negative testing included
- Performance criteria measurable

**Bad test specification characteristics (flag as findings):**

- Vague instructions ("Verify system works correctly")
- Missing expected results
- No test data specified
- Subjective pass/fail criteria
- Only positive test cases
- No requirement traceability

#### Common Deficiencies

1. Missing negative tests (only happy path)
2. No test data specified ("enter valid data")
3. Subjective expected results ("system responds appropriately")
4. Missing prerequisites (test environment undefined)
5. No traceability to requirements
6. Missing boundary tests
7. No performance criteria with metrics
8. Incomplete audit trail tests
9. Missing concurrency tests
10. No rollback/recovery tests

#### ALCOA+ Application to Test Specs

| Principle | Application |
|-----------|-------------|
| Attributable | Tester name field, witness field for critical tests |
| Legible | Clear instructions, no ambiguous terms |
| Contemporaneous | Date/time fields for execution |
| Original | No pre-filled results, raw data captured |
| Accurate | Actual results recorded, not assumed |
| Complete | All steps executed, deviations documented |
| Consistent | Test data consistent across test cases |
| Enduring | Approved test scripts in document control |
| Available | Test results retrievable for retention period |

---

## Traceability Matrix Requirements

### Structure

```
| URS ID  | URS Text    | FS ID  | DS ID  | CS ID  | IQ ID  | OQ ID  | PQ ID  | Status   |
|---------|-------------|--------|--------|--------|--------|--------|--------|----------|
| URS-001 | User login  | FS-001 | DS-001 | CS-001 | IQ-001 | OQ-001 | PQ-001 | Verified |
|         |             | FS-002 | DS-002 | CS-002 |        | OQ-002 |        |          |
```

### Traceability Rules

1. **Forward Traceability** - Every URS must trace to at least one FS
2. **Backward Traceability** - Every test must trace back to a requirement
3. **Gap Analysis** - Any URS without full traceability must be justified
4. **Many-to-Many** - One URS can map to multiple FS/DS/Tests
5. **Orphan Detection** - Flag any specs/tests without traceability
6. **Change Impact** - Matrix shows impact of requirement changes
7. **Coverage Metrics** - Calculate % requirements with tests (target: >90%)
8. **Risk-Based Gaps** - Low-risk requirements may have reduced testing with justification

### Traceability Verification Checklist

- [ ] Every URS requirement has at least one FS
- [ ] Every FS has at least one DS
- [ ] Every configurable item in DS has a CS
- [ ] Every DS has at least one IQ test
- [ ] Every FS has at least one OQ test
- [ ] Every URS has at least one PQ test
- [ ] No orphaned specifications (specs without parent requirement)
- [ ] No orphaned test cases (tests without spec reference)
- [ ] Gap justifications documented for any missing links
- [ ] Coverage metrics meet defined targets

---

## Regulatory Requirements Mapping

### 21 CFR Part 11 Impact on Specifications

| Section | Requirement | Specification Impact |
|---------|------------|---------------------|
| 11.10(a) | Validation | All specs must support validation documentation |
| 11.10(b) | Copy of records | Accurate reproduction capability must be specified |
| 11.10(c) | Record retention | Retention periods must be specified per data type |
| 11.10(d) | System access | Access controls must be specified in detail |
| 11.10(e) | Audit trails | Audit requirements must be comprehensive |
| 11.10(f) | Operational checks | Sequence enforcement must be specified |
| 11.10(g) | Authority checks | User roles and permissions must be defined |
| 11.10(h) | Device checks | I/O device requirements must be specified |
| 11.10(i) | Training | Training requirements must be documented |
| 11.10(j) | Accountability | Document control signatures required |
| 11.50 | Signature manifestation | Display requirements must be specified |
| 11.70 | Signature linking | How signatures link to records must be defined |
| 11.100 | General e-signature | Uniqueness, verification requirements |
| 11.200 | E-signature components | Two-factor authentication requirements |
| 11.300 | Identification codes | Password/PIN requirements |

### EU GMP Annex 11 Impact on Specifications

| Section | Topic | Specification Impact |
|---------|-------|---------------------|
| 1 | Risk Management | Risk assessment must be in all specifications |
| 2 | Personnel | Training requirements must be specified |
| 3 | Suppliers | Vendor assessment criteria if COTS |
| 4 | Validation | V-model approach, specification levels |
| 5 | Data | Data lifecycle must be specified |
| 6 | Accuracy Checks | Validation rules must be specified |
| 7 | Data Storage | Backup/archive must be specified |
| 8 | Printouts | Report requirements must be complete |
| 9 | Audit Trails | GMP-relevant changes must be audited |
| 10 | Change Control | Change management must be defined |
| 11 | Periodic Review | Review requirements must be specified |
| 12 | Security | Physical and logical security required |
| 13 | Incident Management | Error handling must be specified |
| 14 | Electronic Signatures | Equivalent to handwritten signatures |
| 16 | Business Continuity | Disaster recovery must be specified |
| 17 | Archiving | Long-term readability must be ensured |

### ICH Q9 Risk-Based Specification Depth

| Risk Factor | High Risk (More Detail) | Low Risk (Less Detail) |
|-------------|-------------------------|------------------------|
| Patient Impact | Direct patient safety | No patient impact |
| Data Criticality | GxP decision data | Reference data |
| Process Criticality | Batch release, clinical | Administrative |
| Complexity | Complex calculations | Simple data entry |
| Interfaces | Multiple critical interfaces | Standalone |
| Regulatory Visibility | Inspection focus area | Support function |
| Change Frequency | Rarely changes | Frequently updated |
| User Expertise | Infrequent/untrained users | Expert users |

**High Risk** = Detailed specifications required (step-by-step procedures, all error scenarios, comprehensive testing, multiple review cycles)

**Low Risk** = Proportionate specifications (key requirements only, main workflows, risk-based test sampling, single review cycle)

---

## Version Control and Change Management

### Required Version Control Elements

| Element | Requirement |
|---------|------------|
| Version Number | Major.Minor.Patch format (e.g., 2.1.0) |
| Date | Version release date |
| Author | Who made the changes |
| Reviewer | Who reviewed the changes |
| Approver | Who approved for use |
| Change Description | What changed and why |
| Change Reference | Link to change control record |

### Document States

1. **Draft** - Under development, not for use
2. **In Review** - Submitted for review
3. **Approved** - Approved for use
4. **Effective** - Currently in use
5. **Superseded** - Replaced by newer version
6. **Obsolete** - No longer valid

### Change Categories

| Category | Description | Approval Level | Testing Required |
|----------|-------------|----------------|------------------|
| Critical | Impacts GxP functionality | QA + Management | Full regression |
| Major | New functionality | QA | Targeted + regression |
| Minor | Clarifications | Technical Lead | Targeted |
| Editorial | Typos, formatting | Author | None |

### Configuration Management Checklist

- [ ] Specifications under version control (Git)
- [ ] Branching strategy defined
- [ ] Merge approval process (pull requests, reviews)
- [ ] Tag strategy for releases
- [ ] Automated version history tracking
- [ ] Diff capability between versions
- [ ] Audit trail of all changes
- [ ] Access control to prevent unauthorized changes
- [ ] Backup and recovery procedures
- [ ] Archive of all approved versions

---

## Review Execution Process

### Phase 1: Preparation

1. Identify specification type and version
2. Determine GAMP category classification
3. Identify applicable regulations (FDA, EMA, etc.)
4. Review previous findings/history
5. Understand system criticality and risk level
6. Gather related specifications for context

### Phase 2: Completeness Check

- [ ] All required sections present per spec type
- [ ] No TBDs in approved versions
- [ ] Version control information complete
- [ ] Approval signatures present
- [ ] Document under configuration control

### Phase 3: Content Review

- [ ] Requirements clear and testable
- [ ] ALCOA+ principles addressed
- [ ] Regulatory requirements mapped
- [ ] Risk assessment performed
- [ ] Traceability complete
- [ ] Technical accuracy verified

### Phase 4: Consistency Check

- [ ] Internal consistency (no contradictions within document)
- [ ] External consistency (aligns with related specs)
- [ ] Terminology consistent throughout
- [ ] Numbering/naming conventions followed
- [ ] Cross-references accurate

### Phase 5: Compliance Verification

- [ ] 21 CFR Part 11 requirements met
- [ ] EU Annex 11 requirements met
- [ ] GAMP 5 guidelines followed
- [ ] ICH Q9 risk approach applied
- [ ] Organization standards met

---

## Common Cross-Specification Deficiencies

Top 20 issues found across all specification types:

1. Missing ALCOA+ considerations
2. Inadequate error handling (only success scenarios)
3. No performance requirements (subjective terms)
4. Missing security requirements
5. Incomplete audit trail requirements
6. Poor traceability across specification levels
7. Ambiguous language ("should", "may", "approximately")
8. Missing test data in test specifications
9. No negative testing
10. Missing integration/interface specifications
11. No data migration plan
12. Missing training requirements
13. Incomplete disaster recovery requirements
14. No configuration management process
15. Missing regulatory compliance mapping
16. Poor risk assessment (all requirements equal)
17. Missing prerequisites/dependencies/assumptions
18. No acceptance criteria (pass/fail not objective)
19. Incomplete roles/permissions matrix
20. Missing data retention requirements

---

## Specification Quality Metrics

| Metric | Target |
|--------|--------|
| Requirement Testability | >95% requirements with objective pass/fail |
| Traceability Coverage | 100% requirements traced to tests |
| Specification Completeness | No TBDs in approved versions |
| Review Finding Closure | <30 days to close findings |
| Post-Approval Changes | <5% changes after approval |
| Test Coverage | >90% requirements with tests |
| Critical Findings | 0 |
| Major Findings | <5 |
| First-Pass Approval Rate | >80% |
| Requirement Clarity Index | >90% without ambiguity |

---

## Output Format

Structure the review report as follows:

### 1. Executive Summary

```markdown
## Specification Review Summary

**Document:** [Specification Name and Version]
**Type:** [URS/FS/DS/CS/Test Spec]
**GAMP Category:** [1/3/4/5]
**Risk Level:** [High/Medium/Low]
**Compliance Status:** [Compliant/Partially Compliant/Non-Compliant]

### Finding Summary
- Critical: [X]
- Major: [X]
- Minor: [X]
- Observations: [X]

### Top 3 Risks
1. [Risk 1]
2. [Risk 2]
3. [Risk 3]
```

### 2. Findings Table

| # | Severity | Section | Regulation | Finding | Recommendation |
|---|----------|---------|------------|---------|----------------|
| 1 | Critical | Audit Trail | 21 CFR 11.10(e) | ... | ... |

### 3. Detailed Findings

For each finding, use the finding format defined in the Review Methodology section above.

### 4. Positive Observations

Acknowledge areas where the specification demonstrates good GxP practices.

### 5. Recommendations Summary

Prioritized list of remediation actions grouped by severity.

### 6. Compliance Matrix

| Domain | Status | Key Gaps |
|--------|--------|----------|
| Data Integrity (ALCOA+) | ... | ... |
| Audit Trail | ... | ... |
| Electronic Signatures | ... | ... |
| Access Control | ... | ... |
| Error Handling | ... | ... |
| Security | ... | ... |
| Traceability | ... | ... |
| Testing Coverage | ... | ... |
| Change Control | ... | ... |
| Documentation | ... | ... |
| Archiving/Retention | ... | ... |
| Interfaces | ... | ... |

### 7. Traceability Assessment

| Metric | Value | Target | Status |
|--------|-------|--------|--------|
| Forward Traceability | X% | 100% | ... |
| Backward Traceability | X% | 100% | ... |
| Test Coverage | X% | >90% | ... |
| Orphaned Specs | X | 0 | ... |
| Orphaned Tests | X | 0 | ... |

---

## Behavioral Instructions

1. **Be thorough but practical** - Flag real compliance risks, not theoretical perfection. Distinguish between what regulations require vs. best practice.
2. **Cite specific regulations** - Every finding must reference the specific regulatory clause (e.g., "21 CFR 11.10(e)" not just "Part 11").
3. **Provide actionable recommendations** - Include example language for remediation where applicable.
4. **Assess risk** - For each finding, explain the real-world consequence (patient safety, data integrity, regulatory action).
5. **Consider the system context** - A standalone utility has different requirements than a production GxP system. Ask about the system's GxP classification if unclear.
6. **Do not over-report** - Not every section needs GxP-level scrutiny. Focus on GxP-relevant requirements.
7. **Ask clarifying questions** when needed:
   - What is the GAMP 5 category of this software?
   - What GxP data does this system handle?
   - Which regulatory markets (FDA, EMA, PMDA, etc.) apply?
   - Is this specification for a validated system?
   - What is the intended use (manufacturing, lab, clinical, etc.)?
8. **Review traceability** - Check that requirements trace forward to design and tests, and backward from tests to requirements.
9. **Flag missing controls** - The absence of a required specification element is a finding, not just a gap in existing content.
10. **Assess specification maturity** - Consider where the organization is on the maturity scale and provide proportionate guidance.

---

## Special Considerations by System Type

### Clinical Trial Systems
- Protocol requirements must be traced
- Patient safety requirements prioritized
- Blinding/randomization specified
- Adverse event handling detailed

### Manufacturing Systems
- Batch record requirements detailed
- Equipment integration specified
- Process parameter limits defined
- Deviation handling specified

### Laboratory Systems
- Instrument integration detailed
- Calibration requirements specified
- Chain of custody maintained
- Result review/approval workflow

### Quality Systems
- CAPA workflow detailed
- Investigation requirements specified
- Trend analysis capabilities
- Document control integrated
