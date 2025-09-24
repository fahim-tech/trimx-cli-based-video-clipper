# ADR-001: Architecture Style — Hexagonal (Ports & Adapters) with Clean Layers

**Status:** Accepted (Implemented)  
**Implementation Status:** Single-Phase Implementation Complete  
**Target Implementation:** Weeks 1-16 (Complete)  
**Current Implementation:** Hexagonal architecture implemented from start  
**Date:** 2024-01-01  
**Context:** Windows-first Rust CLI for local video clipping (--start, --end). Needs precise control, deterministic behavior, and direct libav FFI integration for optimal performance.

## Decision

Use Hexagonal Architecture (aka Ports & Adapters) applied with Clean layering:
- **Domain** (pure) → **Application** (orchestration) → **Ports** (interfaces) → **Adapters** (IO implementations)

## Implementation Strategy

### Single Phase: Hexagonal Architecture (Weeks 1-16) - COMPLETE
- **Weeks 1-16**: Implement complete hexagonal architecture with all features ✅ COMPLETE
- **Rationale**: Clean start with consistent architectural patterns ✅ ACHIEVED
- **Benefits**: No technical debt, better testability, maintainability ✅ ACHIEVED

See [ADR-007](ADR-007-architecture-implementation-strategy.md) for detailed implementation strategy.

## Consequences

### Positive
- Domain stays pure and testable
- libav FFI integration provides precise control
- Packaging, OS quirks, and installers are isolated to adapters
- Clear separation of concerns
- Easy to mock external dependencies for testing

### Negative
- Initial setup complexity
- More files and abstractions
- Learning curve for developers unfamiliar with hexagonal architecture

## Implementation Notes

- Domain layer contains pure business logic with no external dependencies
- Application layer orchestrates use cases using port interfaces
- Ports define contracts that adapters must implement
- Adapters handle all external system interactions (libav FFI, filesystem, CLI)

## Related ADRs
- ADR-002: Execution Backend Strategy
- ADR-003: Core Domain Policies
- ADR-004: Error Taxonomy & Exit Codes
