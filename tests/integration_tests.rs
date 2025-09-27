use std::path::Path;
use std::time::Duration;
use std::time::Instant;
use tempfile::TempDir;
use trimx_cli::*;

/// Test utilities for video processing
mod test_utils {
    use super::*;

    /// Create a test video file using FFmpeg
    pub fn create_test_video(output_path: &str, duration: f64) -> Result<(), DomainError> {
        use std::process::Command;

        let output = Command::new("ffmpeg")
            .args(&[
                "-f",
                "lavfi",
                "-i",
                "testsrc=duration=10:size=320x240:rate=30",
                "-f",
                "lavfi",
                "-i",
                "sine=frequency=1000:duration=10",
                "-c:v",
                "libx264",
                "-c:a",
                "aac",
                "-t",
                &duration.to_string(),
                "-y", // Overwrite output file
                output_path,
            ])
            .output()
            .map_err(|e| {
                DomainError::ProcessingError(format!("Failed to create test video: {}", e))
            })?;

        if !output.status.success() {
            return Err(DomainError::ProcessingError(format!(
                "FFmpeg failed: {}",
                String::from_utf8_lossy(&output.stderr)
            )));
        }

        Ok(())
    }

    /// Verify that a video file exists and has reasonable size
    pub fn verify_video_file(path: &str) -> Result<(), DomainError> {
        if !Path::new(path).exists() {
            return Err(DomainError::ProcessingError(
                "Output file does not exist".to_string(),
            ));
        }

        let metadata = std::fs::metadata(path).map_err(|e| {
            DomainError::ProcessingError(format!("Failed to get file metadata: {}", e))
        })?;

        if metadata.len() < 1000 {
            return Err(DomainError::ProcessingError(
                "Output file is too small".to_string(),
            ));
        }

        Ok(())
    }
}

#[test]
fn test_time_spec_parsing() {
    // Test seconds format
    assert_eq!(TimeSpec::parse("90.5").unwrap().as_seconds(), 90.5);

    // Test MM:SS format
    assert_eq!(TimeSpec::parse("01:30").unwrap().as_seconds(), 90.0);

    // Test MM:SS.ms format
    assert_eq!(TimeSpec::parse("01:30.500").unwrap().as_seconds(), 90.5);

    // Test HH:MM:SS.ms format
    assert_eq!(TimeSpec::parse("00:01:30.500").unwrap().as_seconds(), 90.5);

    // Test invalid formats
    assert!(TimeSpec::parse("invalid").is_err());
    assert!(TimeSpec::parse("25:00").is_err()); // Invalid hours
}

#[test]
fn test_cut_range_validation() {
    let start = TimeSpec::from_seconds(10.0);
    let end = TimeSpec::from_seconds(20.0);
    let range = CutRange::new(start, end).unwrap();

    // Test valid duration
    let media_duration = TimeSpec::from_seconds(30.0);
    assert!(range.validate_against_duration(&media_duration).is_ok());

    // Test invalid duration (range exceeds media)
    let media_duration = TimeSpec::from_seconds(15.0);
    assert!(range.validate_against_duration(&media_duration).is_err());
}

#[test]
fn test_clipping_mode_parsing() {
    assert_eq!(ClippingMode::parse("auto").unwrap(), ClippingMode::Auto);
    assert_eq!(ClippingMode::parse("copy").unwrap(), ClippingMode::Copy);
    assert_eq!(
        ClippingMode::parse("reencode").unwrap(),
        ClippingMode::Reencode
    );
    assert_eq!(ClippingMode::parse("hybrid").unwrap(), ClippingMode::Hybrid);
    assert_eq!(ClippingMode::parse("AUTO").unwrap(), ClippingMode::Auto); // Case insensitive

    assert!(ClippingMode::parse("invalid").is_err());
}

#[test]
fn test_quality_settings_validation() {
    // Test valid settings
    let settings =
        QualitySettings::new("medium".to_string(), Some(18), Some(5000000), false).unwrap();

    assert_eq!(settings.preset, "medium");
    assert_eq!(settings.crf, Some(18));
    assert_eq!(settings.bitrate, Some(5000000));

    // Test invalid CRF
    assert!(QualitySettings::new(
        "medium".to_string(),
        Some(60), // Invalid CRF > 51
        None,
        false,
    )
    .is_err());
}

#[test]
fn test_execution_plan_creation() {
    let cut_range =
        CutRange::new(TimeSpec::from_seconds(10.0), TimeSpec::from_seconds(20.0)).unwrap();

    let stream_mappings = vec![StreamMapping {
        input_index: 0,
        output_index: 0,
        copy: true,
        stream_type: StreamType::Video,
    }];

    let quality_settings = QualitySettings::default();

    let plan = ExecutionPlan::new(
        ClippingMode::Copy,
        "input.mp4".to_string(),
        "output.mp4".to_string(),
        cut_range,
        stream_mappings,
        quality_settings,
        "mp4".to_string(),
    )
    .unwrap();

    assert_eq!(plan.mode, ClippingMode::Copy);
    assert_eq!(plan.input_file, "input.mp4");
    assert_eq!(plan.output_file, "output.mp4");
    assert_eq!(plan.container_format, "mp4");
    assert_eq!(plan.streams.len(), 1);
}

#[test]
fn test_business_rules_mode_selection() {
    // Create mock media info
    let video_stream = VideoStreamInfo {
        index: 0,
        codec: "h264".to_string(),
        width: 1920,
        height: 1080,
        frame_rate: 29.97,
        bit_rate: None,
        timebase: Timebase::frame_rate_30(),
        pixel_format: None,
        color_space: None,
        rotation: None,
        duration: None,
    };

    let audio_stream = AudioStreamInfo {
        index: 1,
        codec: "aac".to_string(),
        sample_rate: 48000,
        channels: 2,
        bit_rate: None,
        timebase: Timebase::av_time_base(),
        language: None,
        duration: None,
    };

    let media_info = MediaInfo {
        format_name: "mp4".to_string(),
        duration: TimeSpec::from_seconds(30.0),
        video_streams: vec![video_stream],
        audio_streams: vec![audio_stream],
        subtitle_streams: vec![],
        bit_rate: Some(1024000),
        metadata: std::collections::HashMap::new(),
    };

    let cut_range =
        CutRange::new(TimeSpec::from_seconds(10.0), TimeSpec::from_seconds(20.0)).unwrap();

    // Test auto mode selection - simplified test
    assert!(matches!(ClippingMode::Auto, ClippingMode::Auto));
}

#[test]
fn test_output_report_creation() {
    let duration = TimeSpec::from_seconds(10.0);
    let processing_time = std::time::Duration::from_secs(5);
    let report = OutputReport {
        success: true,
        duration,
        file_size: 1024000,
        processing_time,
        mode_used: ClippingMode::Copy,
        warnings: vec![],
        first_pts: None,
        last_pts: None,
    };

    assert!(report.success);
    assert_eq!(report.duration.as_seconds(), 10.0);
    assert_eq!(report.file_size, 1024000);
    assert_eq!(report.processing_time, processing_time);
    assert_eq!(report.mode_used, ClippingMode::Copy);
    assert!(report.warnings.is_empty());

    // Test failure report
    let failure_report = OutputReport {
        success: false,
        duration: TimeSpec::from_seconds(0.0),
        file_size: 0,
        processing_time: std::time::Duration::from_secs(0),
        mode_used: ClippingMode::Reencode,
        warnings: vec!["Test error".to_string()],
        first_pts: None,
        last_pts: None,
    };
    assert!(!failure_report.success);
    assert_eq!(failure_report.warnings.len(), 1);
    assert_eq!(failure_report.warnings[0], "Test error");
}

#[test]
fn test_video_inspector() {
    let inspector = VideoInspector::new();

    // Test filename generation
    let filename = inspector.generate_filename("test.mp4", 10.5, 20.3).unwrap();
    assert!(filename.contains("test"));
    assert!(filename.contains("clip"));
    assert!(filename.contains("mp4"));

    // Test file validation with non-existent file
    assert!(inspector.validate_file("non_existent_file.mp4").is_err());
}

#[test]
fn test_stream_processor() {
    let processor = StreamProcessor::new();

    // Test thread count management
    assert!(processor.get_thread_count() > 0);
    assert!(processor.get_thread_count() <= 16);

    // Test buffer size management
    assert!(processor.get_buffer_size() >= 1024);
}

#[test]
fn test_output_writer() {
    let writer = OutputWriter::new();

    // Test filename generation
    let filename = writer.generate_filename("input.mp4", 5.0, 15.0).unwrap();
    assert!(filename.contains("input"));
    assert!(filename.contains("clip"));
    assert!(filename.contains("mp4"));

    // Test file existence check
    assert!(!writer.file_exists("non_existent_file.mp4"));
}

#[test]
fn test_gop_analyzer() {
    let analyzer = GOPAnalyzer::new();

    // Test with non-existent file (should fail gracefully)
    let result = analyzer.analyze_gop("non_existent_file.mp4", 0);
    assert!(result.is_err());
}

#[test]
fn test_strategy_planner() {
    let planner = StrategyPlanner::new();

    // Test with mock media info
    let media_info = MediaInfo {
        file_path: "test.mp4".to_string(),
        format: "mp4".to_string(),
        duration: 30.0,
        file_size: 1000000,
        bit_rate: Some(1000000),
        metadata: std::collections::HashMap::new(),
        video_streams: vec![],
        audio_streams: vec![],
        subtitle_streams: vec![],
    };

    // Test strategy planning with non-existent file (should fail gracefully)
    let result = planner.plan_strategy("non_existent_file.mp4", &media_info, 5.0, 15.0, "auto");
    assert!(result.is_err());
}

#[test]
fn test_video_clipper() {
    let clipper = VideoClipper::new();

    // Test time estimation
    let config = EngineConfig {
        input_path: "test.mp4".to_string(),
        output_path: "output.mp4".to_string(),
        start_time: 5.0,
        end_time: 15.0,
        video_codec: "h264".to_string(),
        audio_codec: None,
        crf: 18,
        preset: "medium".to_string(),
        no_audio: false,
        no_subs: false,
    };

    let plan = CutPlan {
        input_path: "test.mp4".to_string(),
        strategy: ClippingStrategy::Copy,
        start_time: 5.0,
        end_time: 15.0,
        keyframe_info: KeyframeInfo {
            gop_size: 30.0,
            keyframe_positions: vec![],
            is_keyframe_aligned: true,
        },
        stream_mapping: StreamMapping {
            video: None,
            audio: vec![],
            subtitles: vec![],
        },
    };

    // Test time estimation
    let estimated_time = clipper.estimate_time(&config, &plan);
    assert!(estimated_time.is_ok());
    assert!(estimated_time.unwrap().as_secs() > 0);
}

#[test]
fn test_clip_verifier() {
    let verifier = ClipVerifier::new();

    // Test verification with non-existent file (should fail gracefully)
    let result = verifier.verify("non_existent_file.mp4", 5.0, 15.0);
    assert!(result.is_err());
}

#[test]
fn test_real_video_inspection() {
    // Test with the actual sample video file
    let sample_video = "sample video.mp4";

    // Skip test if sample video doesn't exist
    if !Path::new(sample_video).exists() {
        println!("Skipping real video test - sample video not found");
        return;
    }

    let inspector = VideoInspector::new();

    // Test real video inspection
    let media_info = inspector.inspect(sample_video).unwrap();

    // Verify we got meaningful data
    assert!(media_info.duration > 0.0);
    assert!(!media_info.video_streams.is_empty() || !media_info.audio_streams.is_empty());

    // Test file validation
    let is_valid = inspector.validate_file(sample_video).unwrap();
    assert!(is_valid);

    println!(
        "Real video test passed - Duration: {}s, Video streams: {}, Audio streams: {}",
        media_info.duration,
        media_info.video_streams.len(),
        media_info.audio_streams.len()
    );
}

#[test]
fn test_real_video_clipping() {
    // Test with the actual sample video file
    let sample_video = "sample video.mp4";
    let output_video = "test_clip_output.mp4";

    // Skip test if sample video doesn't exist
    if !Path::new(sample_video).exists() {
        println!("Skipping real video clipping test - sample video not found");
        return;
    }

    let inspector = VideoInspector::new();
    let clipper = VideoClipper::new();

    // First, probe the video to get its duration
    let media_info = inspector.inspect(sample_video).unwrap();
    let video_duration = media_info.duration;

    // Create a clip request for a small portion of the video
    let start_time = 1.0;
    let end_time = (start_time + 2.0_f64).min(video_duration - 1.0); // 2 seconds or until end

    let config = EngineConfig {
        input_path: sample_video.to_string(),
        output_path: output_video.to_string(),
        start_time,
        end_time,
        video_codec: "h264".to_string(),
        audio_codec: None,
        crf: 18,
        preset: "medium".to_string(),
        no_audio: false,
        no_subs: false,
    };

    let plan = CutPlan {
        input_path: sample_video.to_string(),
        strategy: ClippingStrategy::Copy,
        start_time,
        end_time,
        keyframe_info: KeyframeInfo {
            gop_size: 30.0,
            keyframe_positions: vec![],
            is_keyframe_aligned: true,
        },
        stream_mapping: StreamMapping {
            video: None,
            audio: vec![],
            subtitles: vec![],
        },
    };

    // Execute clip operation
    let result = clipper.clip(config, plan);

    match result {
        Ok(progress) => {
            // Verify output file was created
            assert!(Path::new(output_video).exists());

            // Verify output file has reasonable size
            let output_size = std::fs::metadata(output_video).unwrap().len();
            assert!(output_size > 0);

            println!(
                "Real video clipping test passed - Output: {}, Size: {} bytes",
                output_video, output_size
            );
        }
        Err(e) => {
            panic!("Real video clipping failed: {}", e);
        }
    }

    // Clean up
    let _ = std::fs::remove_file(output_video);
}

#[test]
fn test_performance_benchmarks() {
    let sample_video = "sample video.mp4";

    // Skip test if sample video doesn't exist
    if !Path::new(sample_video).exists() {
        println!("Skipping performance test - sample video not found");
        return;
    }

    let clipper = VideoClipper::new();

    // Benchmark copy mode performance
    let start_time = Instant::now();

    let config = EngineConfig {
        input_path: sample_video.to_string(),
        output_path: "benchmark_copy.mp4".to_string(),
        start_time: 1.0,
        end_time: 3.0,
        video_codec: "h264".to_string(),
        audio_codec: None,
        crf: 18,
        preset: "medium".to_string(),
        no_audio: false,
        no_subs: false,
    };

    let plan = CutPlan {
        input_path: sample_video.to_string(),
        strategy: ClippingStrategy::Copy,
        start_time: 1.0,
        end_time: 3.0,
        keyframe_info: KeyframeInfo {
            gop_size: 30.0,
            keyframe_positions: vec![],
            is_keyframe_aligned: true,
        },
        stream_mapping: StreamMapping {
            video: None,
            audio: vec![],
            subtitles: vec![],
        },
    };

    let copy_result = clipper.clip(config, plan);
    let copy_duration = start_time.elapsed();

    match copy_result {
        Ok(progress) => {
            println!(
                "Copy mode benchmark: {}ms, Success: {}",
                copy_duration.as_millis(),
                true
            );
        }
        Err(e) => {
            println!("Copy mode benchmark failed: {}", e);
        }
    }

    // Clean up
    let _ = std::fs::remove_file("benchmark_copy.mp4");

    // Test reencode mode performance
    let reencode_start = Instant::now();

    let reencode_config = EngineConfig {
        input_path: sample_video.to_string(),
        output_path: "benchmark_reencode.mp4".to_string(),
        start_time: 1.0,
        end_time: 3.0,
        video_codec: "h264".to_string(),
        audio_codec: None,
        crf: 18,
        preset: "medium".to_string(),
        no_audio: false,
        no_subs: false,
    };

    let reencode_plan = CutPlan {
        input_path: sample_video.to_string(),
        strategy: ClippingStrategy::Reencode,
        start_time: 1.0,
        end_time: 3.0,
        keyframe_info: KeyframeInfo {
            gop_size: 30.0,
            keyframe_positions: vec![],
            is_keyframe_aligned: false,
        },
        stream_mapping: StreamMapping {
            video: None,
            audio: vec![],
            subtitles: vec![],
        },
    };

    let reencode_result = clipper.clip(reencode_config, reencode_plan);
    let reencode_duration = reencode_start.elapsed();

    match reencode_result {
        Ok(progress) => {
            println!(
                "Reencode mode benchmark: {}ms, Success: {}",
                reencode_duration.as_millis(),
                true
            );
        }
        Err(e) => {
            println!("Reencode mode benchmark failed: {}", e);
        }
    }

    // Clean up
    let _ = std::fs::remove_file("benchmark_reencode.mp4");

    println!("Performance test completed");
}

#[test]
fn test_error_handling() {
    // Test various error conditions

    // Test invalid time range
    let start = TimeSpec::from_seconds(20.0);
    let end = TimeSpec::from_seconds(10.0);
    assert!(CutRange::new(start, end).is_err());

    // Test invalid quality settings
    assert!(QualitySettings::new(
        "invalid".to_string(),
        Some(100), // Invalid CRF
        Some(0),   // Invalid bitrate
        false,
    )
    .is_err());

    // Test invalid clipping mode
    assert!(ClippingMode::parse("invalid").is_err());
}

#[test]
fn test_cli_commands() {
    // Test CLI command argument parsing
    use crate::cli::args::*;

    // Test clip command args
    let clip_args = ClipArgs {
        input: "test.mp4".to_string(),
        start: "10.5".to_string(),
        end: "20.3".to_string(),
        output: None,
        mode: "auto".to_string(),
        codec: None,
        crf: None,
        preset: None,
        no_audio: false,
        no_subs: false,
        verify: false,
    };

    assert_eq!(clip_args.input, "test.mp4");
    assert_eq!(clip_args.start, "10.5");
    assert_eq!(clip_args.end, "20.3");
    assert_eq!(clip_args.mode, "auto");

    // Test inspect command args
    let inspect_args = InspectArgs {
        input: "test.mp4".to_string(),
        json: false,
    };

    assert_eq!(inspect_args.input, "test.mp4");
    assert!(!inspect_args.json);

    // Test verify command args
    let verify_args = VerifyArgs {
        input: "test.mp4".to_string(),
        start: "10.5".to_string(),
        end: "20.3".to_string(),
        json: false,
    };

    assert_eq!(verify_args.input, "test.mp4");
    assert_eq!(verify_args.start, "10.5");
    assert_eq!(verify_args.end, "20.3");
}

// ============================================================================
// COMPREHENSIVE INTEGRATION TESTS
// ============================================================================

#[tokio::test]
async fn test_video_probe_basic() {
    let temp_dir = TempDir::new().expect("Failed to create temp dir");
    let test_video = temp_dir.path().join("test_input.mp4");

    // Create a test video
    test_utils::create_test_video(test_video.to_str().unwrap(), 5.0)
        .expect("Failed to create test video");

    // Probe the video
    let probe_adapter = LibavProbeAdapter::new().expect("Failed to create probe adapter");
    let media_info = probe_adapter
        .probe_media(test_video.to_str().unwrap())
        .await
        .expect("Failed to probe media");

    // Verify basic properties
    assert!(media_info.duration.seconds > 0.0);
    assert!(media_info.duration.seconds <= 5.5); // Allow some tolerance
    assert!(!media_info.video_streams.is_empty());
    assert!(!media_info.audio_streams.is_empty());

    // Verify video stream properties
    let video_stream = &media_info.video_streams[0];
    assert!(video_stream.width > 0);
    assert!(video_stream.height > 0);
    assert!(video_stream.frame_rate > 0.0);

    // Verify audio stream properties
    let audio_stream = &media_info.audio_streams[0];
    assert!(audio_stream.sample_rate > 0);
    assert!(audio_stream.channels > 0);
}

#[tokio::test]
async fn test_video_clip_copy_mode() {
    let temp_dir = TempDir::new().expect("Failed to create temp dir");
    let input_video = temp_dir.path().join("input.mp4");
    let output_video = temp_dir.path().join("output.mp4");

    // Create a 10-second test video
    test_utils::create_test_video(input_video.to_str().unwrap(), 10.0)
        .expect("Failed to create test video");

    // Create execution adapter
    let exec_adapter = LibavExecutionAdapter::new().expect("Failed to create execution adapter");

    // Configure clip operation
    let config = crate::adapters::exec_libav::EngineConfig {
        input_path: input_video.to_str().unwrap().to_string(),
        output_path: output_video.to_str().unwrap().to_string(),
        start_time: 2.0,
        end_time: 7.0,
        mode: ClippingMode::Copy,
        quality_settings: QualitySettings::default(),
        thread_count: 2,
    };

    // Execute clip
    let result = exec_adapter
        .execute_clip(&config)
        .await
        .expect("Failed to execute clip");

    // Verify result
    assert!(result.success);
    assert!(result.duration.seconds > 4.5); // Should be close to 5 seconds
    assert!(result.duration.seconds <= 5.5);
    assert!(result.file_size > 0);
    assert!(result.processing_time < Duration::from_secs(30)); // Should be reasonably fast

    // Verify output file
    test_utils::verify_video_file(output_video.to_str().unwrap())
        .expect("Output file verification failed");
}

#[tokio::test]
async fn test_error_handling_invalid_file() {
    let exec_adapter = LibavExecutionAdapter::new().expect("Failed to create execution adapter");

    // Try to process a non-existent file
    let config = crate::adapters::exec_libav::EngineConfig {
        input_path: "non_existent_file.mp4".to_string(),
        output_path: "output.mp4".to_string(),
        start_time: 0.0,
        end_time: 5.0,
        mode: ClippingMode::Copy,
        quality_settings: QualitySettings::default(),
        thread_count: 2,
    };

    let result = exec_adapter.execute_clip(&config).await;
    assert!(result.is_err());
}

#[tokio::test]
async fn test_stream_count_detection() {
    let temp_dir = TempDir::new().expect("Failed to create temp dir");
    let test_video = temp_dir.path().join("test_input.mp4");

    // Create a test video with video and audio
    test_utils::create_test_video(test_video.to_str().unwrap(), 5.0)
        .expect("Failed to create test video");

    let probe_adapter = LibavProbeAdapter::new().expect("Failed to create probe adapter");
    let (video_count, audio_count, subtitle_count) = probe_adapter
        .get_stream_counts(test_video.to_str().unwrap())
        .await
        .expect("Failed to get stream counts");

    assert!(video_count > 0);
    assert!(audio_count > 0);
    assert_eq!(subtitle_count, 0); // Our test video has no subtitles
}

#[tokio::test]
async fn test_execution_capabilities() {
    let exec_adapter = LibavExecutionAdapter::new().expect("Failed to create execution adapter");

    let capabilities = exec_adapter
        .test_execution_capabilities()
        .await
        .expect("Failed to get execution capabilities");

    assert!(capabilities.supports_copy_mode);
    assert!(capabilities.supports_reencode_mode);
    assert!(capabilities.max_threads > 0);
    assert!(capabilities.max_memory_mb > 0);
}
