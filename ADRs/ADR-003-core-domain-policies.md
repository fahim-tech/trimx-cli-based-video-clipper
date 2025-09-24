# ADR-003: Core Domain Policies — Timebase, Keyframe, and Plan Selection

**Status:** Accepted (Target Architecture)  
**Implementation Status:** Single-Phase Implementation  
**Target Implementation:** Weeks 1-16  
**Current Implementation:** Hexagonal architecture from start  
**Date:** 2024-01-01  
**Context:** Accurate cuts with reliable defaults while preserving speed when possible.

## Decision

### Timebase Normalization
- Normalize all timing in stream timebase (PTS)
- Avoid frame counts for timing calculations
- Use `Timebase { num, den }` with `rescale_pts()` helpers

### Keyframe Proximity Rule
- Copy allowed if `distance_to_prev_keyframe ≤ ε`
- Where `ε ≈ 0.5 * avg_frame_time`
- Container copy must be safe for the target format

### Plan Modes
1. **Copy** (fast, approximate but safe when aligned)
   - Direct stream copy when cuts align with keyframes
   - Fastest processing, minimal quality loss
   - Fallback to Hybrid if alignment is poor

2. **Hybrid** ("sandwich": re-encode head/tail GOPs, copy middle)
   - Re-encode only necessary leading/trailing GOPs
   - Stream-copy middle segments for speed
   - Balance between speed and accuracy

3. **Reencode** (exact, fallback)
   - Full re-encoding for precise cuts
   - Highest accuracy, slowest processing
   - Used when other modes are unsuitable

### Metadata Preservation
- Preserve rotation/color metadata
- MP4 outputs must have faststart
- Maintain subtitle streams when possible
- Preserve audio/video codec parameters

## Implementation Status

### Current Implementation (v0.1.0)
- **Status**: No domain logic implemented
- **Missing**: Timebase handling
- **Missing**: Keyframe analysis
- **Missing**: Plan selection logic
- **Missing**: Metadata preservation

### Target Implementation (Phase 1)
- **Weeks 1-2**: Basic timebase handling and validation
- **Weeks 3-4**: Copy mode implementation with keyframe analysis
- **Weeks 5-6**: Enhanced error handling and validation
- **Weeks 7-8**: Output validation and quality assurance

## Consequences

### Positive
- Predictable accuracy with performance wins when alignment allows
- Clear decision tree for mode selection
- Maintains video quality while optimizing for speed
- Handles edge cases gracefully

### Negative
- Complex logic for mode selection
- Potential quality variations between modes
- Requires deep understanding of video codecs
- More complex testing scenarios

## Implementation Details

### Mode Selection Algorithm
1. Analyze keyframe positions relative to cut points
2. Calculate alignment scores for each mode
3. Select mode based on accuracy requirements and performance constraints
4. Fallback to more accurate modes if alignment is poor

### Quality Assurance
- Tolerance checks for output duration
- Frame accuracy validation
- Metadata preservation verification
- Performance benchmarking

## Related ADRs
- ADR-001: Architecture Style
- ADR-002: Execution Backend Strategy
- ADR-004: Error Taxonomy & Exit Codes
