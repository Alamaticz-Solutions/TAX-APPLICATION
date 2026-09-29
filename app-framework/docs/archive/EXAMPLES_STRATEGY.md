# Examples & Templates Strategy

*📋 Planning Document • Repository Organization*

This document outlines our strategy for organizing examples and templates to keep the main app_gen framework repository clean and focused.

## Problem Statement

The main app_gen framework repository should remain:
- **Clean and focused** on core framework functionality
- **Easy to maintain** without example code becoming outdated
- **Simple to upgrade** without breaking example dependencies
- **Professional** for developers evaluating the framework

## Solution: Separate Examples Repository

### Repository Structure

**Main Repository (`app-framework`)**:
- Core framework code
- Documentation
- CLI tools and IDE integrations
- Testing infrastructure
- Framework-level features

**Examples Repository (`app-gen-examples`)**:
- Real-world example applications
- Starter templates
- Best practices demonstrations
- Domain-specific implementations

### Examples Repository Organization

```
app-gen-examples/
├── ../README.md                    # Overview and navigation
├── CONTRIBUTING.md              # How to add new examples
├── examples/
│   ├── basic-blog/             # Simple blog with CRUD operations
│   ├── e-commerce/             # Product catalog with complex relationships
│   ├── task-management/        # Project management with user roles
│   ├── dental-practice/        # Complex business logic example
│   ├── social-media/           # Real-time features and subscriptions
│   └── multi-tenant-saas/      # Enterprise patterns
├── starter-templates/
│   ├── minimal-api/            # Bare minimum setup
│   ├── auth-enabled/           # Authentication pre-configured
│   ├── multi-tenant/           # Multi-tenancy patterns
│   └── microservice/           # Service-to-service communication
├── patterns/
│   ├── custom-resolvers/       # GraphQL customization examples
│   ├── business-logic/         # Service layer patterns
│   ├── testing-strategies/     # Comprehensive testing examples
│   └── deployment/             # Production deployment examples
└── docs/
    ├── example-guide.md        # How to use examples effectively
    ├── customization-cookbook.md
    └── migration-examples.md
```

### Benefits of This Approach

**For Framework Maintenance:**
- Core repository stays focused on framework code
- Framework releases don't require updating all examples
- Easier to review and merge framework PRs
- Cleaner issue tracking (framework vs example issues)

**For Developers:**
- Can clone specific examples they need
- Examples have their own documentation and setup
- Clear separation between learning examples and production framework
- Examples can demonstrate different complexity levels

**For Community:**
- Contributors can add examples without touching core framework
- Examples can evolve based on community feedback
- Easier to maintain example-specific documentation
- Examples can target different framework versions

## Implementation Plan

### Phase 1: Repository Setup
1. Create `app-gen-examples` repository
2. Set up initial structure and README
3. Create contribution guidelines for examples

### Phase 2: Content Migration
1. Identify any existing example code in main repository
2. Move to examples repository with proper git history
3. Update main repository to remove example code

### Phase 3: Documentation Updates
1. Update main README to reference examples repository
2. Add examples section to Getting Started guide
3. Update roadmap to reflect new organization
4. Create cross-reference links between repositories

### Phase 4: Example Development
1. Create 2-3 high-quality initial examples
2. Develop starter templates for common use cases
3. Document best practices and patterns
4. Set up CI/CD for example validation

## Cross-Repository Strategy

### Versioning
- Examples repository tags correspond to framework versions
- Each example specifies compatible framework versions
- Clear upgrade paths documented for examples

### Documentation Links
- Main repository links to specific examples for concepts
- Examples reference specific framework documentation sections
- Bidirectional navigation between repositories

### Issue Management
- Framework issues stay in main repository
- Example-specific issues go to examples repository
- Clear guidelines for where to report different types of issues

## Roadmap Item Allocation

### Main Repository
- [x] CLI tool for project scaffolding
- [x] Hot reload for schema changes  
- [x] Better error messages in code generation
- [x] IDE integration (VS Code extension)
- [x] Testing infrastructure and documentation
- [x] Performance testing patterns
- [x] All core framework features

### Examples Repository
- [x] Real-world example applications
- [x] Starter templates for common domains
- [x] Best practices cookbook
- [x] Domain-specific migration guides
- [x] Customization pattern demonstrations

## Success Metrics

**Framework Repository:**
- Faster PR review and merge times
- Cleaner issue tracking
- Easier maintenance and releases
- More focused contributor experience

**Examples Repository:**
- High-quality, working examples
- Active community contributions
- Clear learning progression for developers
- Positive developer feedback on example quality

## Next Steps

1. **Create examples repository** with initial structure
2. **Update main repository documentation** to reference examples
3. **Develop 2-3 initial high-quality examples**
4. **Establish contribution guidelines** for both repositories
5. **Set up cross-repository linking strategy**

This separation will help us maintain a clean, professional framework repository while providing rich examples and templates for developers to learn from and build upon.
