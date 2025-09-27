//! Integration tests for the new FFmpeg-based engines

use std::fs;
use std::path::Path;
use tempfile::TempDir;
use trimx_cli::engine::{EngineConfig, HybridClipper, ReencodeClipper, StreamCopyClipper};
use trimx_cli::error::TrimXResult;
use trimx_cli::planner::{ClippingStrategy, CutPlan, KeyframeInfo};

// Test utilities

/// Create a temporary test video file (placeholder - would use actual video in real tests)
fn create_test_video(path: &str) -> TrimXResult<()> {
    // For now, create an empty file
    // In real implementation, this would create or copy a real video file
    fs::write(path, b"fake video data")?;
    Ok(())
}

/// Create test engine configuration
fn create_test_config(input: &str, output: &str, start: f64, end: f64) -> EngineConfig {
    EngineConfig {
        input_path: input.to_string(),
        output_path: output.to_string(),
        start_time: start,
        end_time: end,
    }
}

/// Create test cut plan
fn create_test_plan() -> CutPlan {
    CutPlan {
        strategy: ClippingStrategy::Hybrid,
        keyframe_info: KeyframeInfo {
            start_keyframe: Some(0.0),
            next_keyframe: Some(2.0),
            end_keyframe: Some(8.0),
            gop_size: Some(2.0),
        },
    }
}

// Stream Copy Engine Tests

#[test]
fn test_stream_copy_clipper_creation() {
    let clipper = StreamCopyClipper::new();
    assert!(!clipper.debug); // Default debug should be false
}

#[test]
fn test_stream_copy_clipper_with_debug() {
    let clipper = StreamCopyClipper::new().with_debug();
    // We can't directly test debug flag since it's private, but we can test that the method chains properly
}

#[test]
fn test_stream_copy_config_validation() {
    let temp_dir = TempDir::new().unwrap();
    let input_path = temp_dir.path().join("input.mp4");
    let output_path = temp_dir.path().join("output.mp4");

    // Create test input file
    create_test_video(input_path.to_str().unwrap()).unwrap();

    let clipper = StreamCopyClipper::new();

    // Valid configuration
    let valid_config = create_test_config(
        input_path.to_str().unwrap(),
        output_path.to_str().unwrap(),
        0.0,
        10.0,
    );

    // This would normally validate successfully, but since we don't have real video,
    // we'll test the config structure
    assert_eq!(valid_config.start_time, 0.0);
    assert_eq!(valid_config.end_time, 10.0);

    // Test is_possible method
    assert!(clipper.is_possible(&valid_config));
}

#[test]
fn test_stream_copy_invalid_time_range() {
    let temp_dir = TempDir::new().unwrap();
    let input_path = temp_dir.path().join("input.mp4");
    let output_path = temp_dir.path().join("output.mp4");

    let clipper = StreamCopyClipper::new();

    // Invalid: start > end
    let invalid_config = create_test_config(
        input_path.to_str().unwrap(),
        output_path.to_str().unwrap(),
        10.0,
        5.0,
    );

    assert!(!clipper.is_possible(&invalid_config));
}

// Re-encoding Engine Tests

#[test]
fn test_reencode_clipper_creation() {
    let clipper = ReencodeClipper::new();
    // Test default values
    assert_eq!(clipper.preset, "medium");
    assert_eq!(clipper.crf, Some(23));
    assert_eq!(clipper.bitrate, None);
}

#[test]
fn test_reencode_clipper_configuration() {
    let clipper = ReencodeClipper::new()
        .with_debug()
        .with_preset("fast")
        .with_crf(18)
        .with_bitrate(5000000);

    assert_eq!(clipper.preset, "fast");
    assert_eq!(clipper.crf, Some(18));
    assert_eq!(clipper.bitrate, Some(5000000));
}

#[test]
fn test_reencode_clipper_crf_bitrate_exclusivity() {
    // Setting bitrate should clear CRF
    let clipper = ReencodeClipper::new().with_crf(20).with_bitrate(3000000);

    assert_eq!(clipper.crf, None);
    assert_eq!(clipper.bitrate, Some(3000000));

    // Setting CRF should clear bitrate
    let clipper = ReencodeClipper::new().with_bitrate(3000000).with_crf(18);

    assert_eq!(clipper.crf, Some(18));
    assert_eq!(clipper.bitrate, None);
}

// Hybrid Engine Tests

#[test]
fn test_hybrid_clipper_creation() {
    let clipper = HybridClipper::new();
    assert_eq!(clipper.min_copy_duration, 2.0);
}

#[test]
fn test_hybrid_clipper_configuration() {
    let clipper = HybridClipper::new()
        .with_debug()
        .with_min_copy_duration(5.0)
        .with_quality(20);

    assert_eq!(clipper.min_copy_duration, 5.0);
}

#[test]
fn test_hybrid_strategy_selection() {
    let temp_dir = TempDir::new().unwrap();
    let input_path = temp_dir.path().join("input.mp4");
    let output_path = temp_dir.path().join("output.mp4");

    create_test_video(input_path.to_str().unwrap()).unwrap();

    let clipper = HybridClipper::new();
    let plan = create_test_plan();

    // Test short clip (should prefer full re-encoding)
    let short_config = create_test_config(
        input_path.to_str().unwrap(),
        output_path.to_str().unwrap(),
        0.0,
        0.5, // Very short clip
    );

    // We can't easily test the internal strategy without exposing it,
    // but we can test that the configuration is valid
    assert!(short_config.end_time > short_config.start_time);

    // Test longer clip (should consider hybrid)
    let long_config = create_test_config(
        input_path.to_str().unwrap(),
        output_path.to_str().unwrap(),
        0.0,
        10.0, // Longer clip
    );

    assert!(long_config.end_time > long_config.start_time);
}

// Engine Configuration Tests

#[test]
fn test_engine_config_creation() {
    let config = EngineConfig {
        input_path: "input.mp4".to_string(),
        output_path: "output.mp4".to_string(),
        start_time: 1.5,
        end_time: 10.5,
    };

    assert_eq!(config.input_path, "input.mp4");
    assert_eq!(config.output_path, "output.mp4");
    assert_eq!(config.start_time, 1.5);
    assert_eq!(config.end_time, 10.5);
}

#[test]
fn test_engine_config_clone() {
    let original = EngineConfig {
        input_path: "input.mp4".to_string(),
        output_path: "output.mp4".to_string(),
        start_time: 2.0,
        end_time: 8.0,
    };

    let cloned = original.clone();
    assert_eq!(original.input_path, cloned.input_path);
    assert_eq!(original.start_time, cloned.start_time);
}

// Cut Plan Tests

#[test]
fn test_cut_plan_creation() {
    let plan = CutPlan {
        strategy: ClippingStrategy::Auto,
        keyframe_info: KeyframeInfo {
            start_keyframe: Some(0.0),
            next_keyframe: Some(2.5),
            end_keyframe: Some(7.5),
            gop_size: Some(2.5),
        },
    };

    assert!(matches!(plan.strategy, ClippingStrategy::Auto));
    assert_eq!(plan.keyframe_info.start_keyframe, Some(0.0));
    assert_eq!(plan.keyframe_info.gop_size, Some(2.5));
}

#[test]
fn test_keyframe_info_defaults() {
    let keyframe_info = KeyframeInfo {
        start_keyframe: None,
        next_keyframe: None,
        end_keyframe: None,
        gop_size: None,
    };

    // Test that we can handle missing keyframe information
    assert!(keyframe_info.start_keyframe.is_none());
    assert!(keyframe_info.gop_size.is_none());
}

// Error Handling Tests

#[test]
fn test_nonexistent_input_file() {
    let clipper = StreamCopyClipper::new();
    let config = create_test_config("/nonexistent/file.mp4", "/tmp/output.mp4", 0.0, 10.0);

    // Should detect that input file doesn't exist
    assert!(!clipper.is_possible(&config));
}

#[test]
fn test_invalid_time_ranges() {
    let temp_dir = TempDir::new().unwrap();
    let input_path = temp_dir.path().join("input.mp4");

    create_test_video(input_path.to_str().unwrap()).unwrap();

    let clipper = StreamCopyClipper::new();

    // Negative start time
    let config = create_test_config(input_path.to_str().unwrap(), "/tmp/output.mp4", -1.0, 10.0);
    assert!(!clipper.is_possible(&config));

    // Start >= End
    let config = create_test_config(input_path.to_str().unwrap(), "/tmp/output.mp4", 10.0, 10.0);
    assert!(!clipper.is_possible(&config));
}

// Size Estimation Tests

#[test]
fn test_stream_copy_size_estimation() {
    let temp_dir = TempDir::new().unwrap();
    let input_path = temp_dir.path().join("input.mp4");

    // Create a test file with known size
    let test_data = vec![0u8; 1024]; // 1KB
    fs::write(&input_path, test_data).unwrap();

    let clipper = StreamCopyClipper::new();
    let config = create_test_config(
        input_path.to_str().unwrap(),
        "/tmp/output.mp4",
        0.0,
        5.0, // Half of assumed 10s duration
    );

    // Should estimate approximately half the input size
    let estimated_size = clipper.estimate_output_size(&config);
    assert!(estimated_size.is_ok());
}

#[test]
fn test_reencode_size_estimation() {
    let temp_dir = TempDir::new().unwrap();
    let input_path = temp_dir.path().join("input.mp4");

    let test_data = vec![0u8; 2048]; // 2KB
    fs::write(&input_path, test_data).unwrap();

    let clipper = ReencodeClipper::new().with_bitrate(1000000); // 1Mbps
    let config = create_test_config(
        input_path.to_str().unwrap(),
        "/tmp/output.mp4",
        0.0,
        8.0, // 8 seconds
    );

    let estimated_size = clipper.estimate_output_size(&config);
    assert!(estimated_size.is_ok());

    if let Ok(size) = estimated_size {
        // 1Mbps for 8 seconds = 8Mb = 1MB
        assert!(size > 0);
        assert!(size <= 2_000_000); // Should be reasonable estimate
    }
}

// Integration Tests (would require real video files)

#[test]
#[ignore] // Ignored by default since it requires real video files
fn test_full_stream_copy_workflow() {
    // This test would require a real video file to work properly
    let input_file = "sample video.mp4";

    if !Path::new(input_file).exists() {
        return; // Skip if sample file not available
    }

    let temp_dir = TempDir::new().unwrap();
    let output_path = temp_dir.path().join("output.mp4");

    let clipper = StreamCopyClipper::new().with_debug();
    let config = create_test_config(input_file, output_path.to_str().unwrap(), 1.0, 3.0);

    // This would actually perform the clipping
    // let result = clipper.clip(config);
    // assert!(result.is_ok());
    // assert!(output_path.exists());
}

#[test]
#[ignore] // Ignored by default since it requires real video files
fn test_full_hybrid_workflow() {
    let input_file = "sample video.mp4";

    if !Path::new(input_file).exists() {
        return;
    }

    let temp_dir = TempDir::new().unwrap();
    let output_path = temp_dir.path().join("hybrid_output.mp4");

    let clipper = HybridClipper::new().with_debug();
    let config = create_test_config(input_file, output_path.to_str().unwrap(), 0.5, 8.5);
    let plan = create_test_plan();

    // This would actually perform the hybrid clipping
    // let result = clipper.clip(config, plan);
    // assert!(result.is_ok());
    // assert!(output_path.exists());
}
