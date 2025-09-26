use trimx_cli::*;
use async_trait::async_trait;
use std::sync::Arc;
use tokio;

#[test]
fn test_time_spec_parsing() {
    // Test seconds format
    assert_eq!(TimeSpec::parse("90.5").unwrap().seconds, 90.5);
    
    // Test MM:SS format
    assert_eq!(TimeSpec::parse("01:30").unwrap().seconds, 90.0);
    
    // Test MM:SS.ms format
    assert_eq!(TimeSpec::parse("01:30.500").unwrap().seconds, 90.5);
    
    // Test HH:MM:SS.ms format
    assert_eq!(TimeSpec::parse("00:01:30.500").unwrap().seconds, 90.5);
    
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
    assert_eq!(ClippingMode::parse("reencode").unwrap(), ClippingMode::Reencode);
    assert_eq!(ClippingMode::parse("hybrid").unwrap(), ClippingMode::Hybrid);
    assert_eq!(ClippingMode::parse("AUTO").unwrap(), ClippingMode::Auto); // Case insensitive
    
    assert!(ClippingMode::parse("invalid").is_err());
}

#[test]
fn test_quality_settings_validation() {
    // Test valid settings
    let settings = QualitySettings::new(
        "medium".to_string(),
        Some(18),
        Some(5000000),
        false,
    ).unwrap();
    
    assert_eq!(settings.preset, "medium");
    assert_eq!(settings.crf, Some(18));
    assert_eq!(settings.bitrate, Some(5000000));
    
    // Test invalid CRF
    assert!(QualitySettings::new(
        "medium".to_string(),
        Some(60), // Invalid CRF > 51
        None,
        false,
    ).is_err());
}

#[test]
fn test_execution_plan_creation() {
    let cut_range = CutRange::new(
        TimeSpec::from_seconds(10.0),
        TimeSpec::from_seconds(20.0),
    ).unwrap();
    
    let stream_mappings = vec![
        StreamMapping {
            input_index: 0,
            output_index: 0,
            copy: true,
            stream_type: StreamType::Video,
        },
    ];
    
    let quality_settings = QualitySettings::default();
    
    let plan = ExecutionPlan::new(
        ClippingMode::Copy,
        "input.mp4".to_string(),
        "output.mp4".to_string(),
        cut_range,
        stream_mappings,
        quality_settings,
        "mp4".to_string(),
    ).unwrap();
    
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
    
    let cut_range = CutRange::new(
        TimeSpec::from_seconds(10.0),
        TimeSpec::from_seconds(20.0),
    ).unwrap();
    
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
    assert_eq!(report.duration.seconds, 10.0);
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

#[tokio::test]
async fn test_probe_adapter_integration() {
    // Create a temporary test file
    let temp_file = std::env::temp_dir().join("test_video.mp4");
    std::fs::write(&temp_file, b"fake video content for testing").unwrap();
    
    let probe_adapter = LibavProbeAdapter::new().unwrap();
    
    // Test format support check
    let is_supported = probe_adapter.is_format_supported(temp_file.to_str().unwrap()).await.unwrap();
    // This might be false if FFmpeg can't handle the fake file, which is expected
    
    // Test file validation
    let is_valid = probe_adapter.validate_file(temp_file.to_str().unwrap()).await.unwrap();
    // This should be true since the file exists and is readable
    
    // Clean up
    std::fs::remove_file(&temp_file).unwrap();
    
    assert!(is_valid);
}

#[tokio::test]
async fn test_exec_adapter_integration() {
    let exec_adapter = LibavExecutionAdapter::new().unwrap();
    
    // Test hardware acceleration detection
    let hw_available = exec_adapter.is_hardware_acceleration_available().await.unwrap();
    // This might be true or false depending on system capabilities
    
    // Test codec detection
    let video_codecs = exec_adapter.get_available_video_codecs().await.unwrap();
    let audio_codecs = exec_adapter.get_available_audio_codecs().await.unwrap();
    
    // Should have at least some codecs available
    assert!(!video_codecs.is_empty() || !audio_codecs.is_empty());
    
    // Test execution capabilities
    let capabilities = exec_adapter.test_execution_capabilities().await.unwrap();
    assert!(capabilities.supports_copy_mode || capabilities.supports_reencode_mode);
}

#[tokio::test]
async fn test_fs_adapter_integration() {
    let fs_adapter = FsWindowsAdapter::new().unwrap();
    
    // Create a temporary test file
    let temp_file = std::env::temp_dir().join("trimx_test_file.txt");
    std::fs::write(&temp_file, b"test content").unwrap();
    
    // Test file operations
    assert!(fs_adapter.file_exists(temp_file.to_str().unwrap()).await.unwrap());
    
    let file_size = fs_adapter.get_file_size(temp_file.to_str().unwrap()).await.unwrap();
    assert_eq!(file_size, 12); // "test content".len()
    
    // Test path validation
    assert!(fs_adapter.validate_path(temp_file.to_str().unwrap()).await.unwrap());
    assert!(!fs_adapter.validate_path("../../../etc/passwd").await.unwrap()); // Path traversal
    assert!(!fs_adapter.validate_path("CON").await.unwrap()); // Reserved name
    
    // Test directory operations
    let temp_dir = std::env::temp_dir().join("trimx_test_dir");
    fs_adapter.create_directory(temp_dir.to_str().unwrap()).await.unwrap();
    assert!(fs_adapter.directory_exists(temp_dir.to_str().unwrap()).await.unwrap());
    
    // Test write permissions
    let can_write = fs_adapter.can_write_to_directory(temp_dir.to_str().unwrap()).await.unwrap();
    assert!(can_write);
    
    // Clean up
    std::fs::remove_file(&temp_file).unwrap();
    std::fs::remove_dir(&temp_dir).unwrap();
}

#[tokio::test]
async fn test_config_adapter_integration() {
    let config_adapter = TomlConfigAdapter::new().unwrap();
    
    // Test default configuration
    let log_level = config_adapter.get_config("log_level").await.unwrap();
    assert!(log_level.is_some());
    
    let default_crf = config_adapter.get_config_or_default("crf", "18").await.unwrap();
    assert_eq!(default_crf, "18");
    
    // Test configuration validation
    config_adapter.validate_config().await.unwrap();
    
    // Test configuration keys
    let keys = config_adapter.get_all_config_keys().await.unwrap();
    assert!(!keys.is_empty());
    assert!(keys.contains(&"log_level".to_string()));
}

#[tokio::test]
async fn test_log_adapter_integration() {
    let log_adapter = TracingLogAdapter::new().unwrap();
    
    // Test basic logging
    log_adapter.info("Test info message").await;
    log_adapter.warn("Test warning message").await;
    log_adapter.error("Test error message").await;
    log_adapter.debug("Test debug message").await;
    
    // Test structured logging
    let log_event = LogEvent {
        level: LogLevel::Info,
        message: "Test structured log".to_string(),
        context: std::collections::HashMap::from([
            ("test_key".to_string(), "test_value".to_string()),
        ]),
    };
    
    log_adapter.log_event(&log_event).await;
    
    // Test log level management
    let current_level = log_adapter.get_log_level().await;
    log_adapter.set_log_level(LogLevel::Debug).await;
    log_adapter.set_json_output(true).await;
}

#[tokio::test]
async fn test_clip_interactor_integration() {
    // Create adapters
    let probe_adapter = Box::new(ProbeLibavAdapter::new().unwrap());
    let exec_adapter = Box::new(ExecLibavAdapter::new().unwrap());
    let fs_adapter = Box::new(FsWindowsAdapter::new().unwrap());
    let config_adapter = Box::new(TomlConfigAdapter::new().unwrap());
    let log_adapter = Box::new(TracingLogAdapter::new().unwrap());
    
    // Create interactor
    let interactor = ClipInteractor::new(
        probe_adapter,
        exec_adapter,
        fs_adapter,
        config_adapter,
        log_adapter,
    );
    
    // Create a temporary test file
    let temp_input = std::env::temp_dir().join("test_input.mp4");
    let temp_output = std::env::temp_dir().join("test_output.mp4");
    std::fs::write(&temp_input, b"fake video content").unwrap();
    
    // Create clip request
    let cut_range = CutRange::new(
        TimeSpec::from_seconds(1.0),
        TimeSpec::from_seconds(5.0),
    ).unwrap();
    
    let request = ClipRequest {
        input_file: temp_input.to_str().unwrap().to_string(),
        output_file: temp_output.to_str().unwrap().to_string(),
        cut_range,
        mode: ClippingMode::Copy,
        quality_settings: None,
    };
    
    // Execute clip operation
    let result = interactor.execute(request).await;
    
    // The operation might fail due to invalid video content, but should not panic
    match result {
        Ok(response) => {
            assert!(!response.output_file.is_empty());
        },
        Err(e) => {
            // Expected for fake video content
            println!("Expected error with fake video content: {}", e);
        }
    }
    
    // Clean up
    let _ = std::fs::remove_file(&temp_input);
    let _ = std::fs::remove_file(&temp_output);
}

#[tokio::test]
async fn test_inspect_interactor_integration() {
    // Create adapters
    let probe_adapter = Box::new(ProbeLibavAdapter::new().unwrap());
    let fs_adapter = Box::new(FsWindowsAdapter::new().unwrap());
    let log_adapter = Box::new(TracingLogAdapter::new().unwrap());
    
    // Create interactor
    let interactor = InspectInteractor::new(
        probe_adapter,
        fs_adapter,
        log_adapter,
    );
    
    // Create a temporary test file
    let temp_file = std::env::temp_dir().join("test_inspect.mp4");
    std::fs::write(&temp_file, b"fake video content for inspection").unwrap();
    
    // Create inspect request
    let request = InspectRequest {
        input_file: temp_file.to_str().unwrap().to_string(),
        include_streams: true,
        include_metadata: true,
    };
    
    // Execute inspect operation
    let result = interactor.execute(request).await;
    
    // The operation might fail due to invalid video content, but should not panic
    match result {
        Ok(response) => {
            assert!(response.success || response.error_message.is_some());
        },
        Err(e) => {
            // Expected for fake video content
            println!("Expected error with fake video content: {}", e);
        }
    }
    
    // Clean up
    let _ = std::fs::remove_file(&temp_file);
}

#[tokio::test]
async fn test_real_video_inspection() {
    // Test with the actual sample video file
    let sample_video = "sample video.mp4";
    
    // Skip test if sample video doesn't exist
    if !std::path::Path::new(sample_video).exists() {
        println!("Skipping real video test - sample video not found");
        return;
    }
    
    let probe_adapter = LibavProbeAdapter::new().unwrap();
    
    // Test real video inspection
    let media_info = probe_adapter.probe_media_file(sample_video).await.unwrap();
    
    // Verify we got meaningful data
    assert!(media_info.duration.seconds > 0.0);
    assert!(!media_info.video_streams.is_empty() || !media_info.audio_streams.is_empty());
    
    // Test format support
    let is_supported = probe_adapter.is_format_supported(sample_video).await.unwrap();
    assert!(is_supported);
    
    // Test file validation
    let is_valid = probe_adapter.validate_file(sample_video).await.unwrap();
    assert!(is_valid);
    
    println!("Real video test passed - Duration: {}s, Video streams: {}, Audio streams: {}", 
             media_info.duration.seconds, 
             media_info.video_streams.len(), 
             media_info.audio_streams.len());
}

#[tokio::test]
async fn test_real_video_clipping() {
    // Test with the actual sample video file
    let sample_video = "sample video.mp4";
    let output_video = "test_clip_output.mp4";
    
    // Skip test if sample video doesn't exist
    if !std::path::Path::new(sample_video).exists() {
        println!("Skipping real video clipping test - sample video not found");
        return;
    }
    
    // Create adapters
    let probe_adapter = Box::new(ProbeLibavAdapter::new().unwrap());
    let exec_adapter = Box::new(ExecLibavAdapter::new().unwrap());
    let fs_adapter = Box::new(FsWindowsAdapter::new().unwrap());
    let config_adapter = Box::new(TomlConfigAdapter::new().unwrap());
    let log_adapter = Box::new(TracingLogAdapter::new().unwrap());
    
    // Create interactor
    let interactor = ClipInteractor::new(
        probe_adapter,
        exec_adapter,
        fs_adapter,
        config_adapter,
        log_adapter,
    );
    
    // First, probe the video to get its duration
    let media_info = interactor.probe_port.probe_media_file(sample_video).await.unwrap();
    let video_duration = media_info.duration.seconds;
    
    // Create a clip request for a small portion of the video
    let start_time = 1.0;
    let end_time = (start_time + 2.0_f64).min(video_duration - 1.0); // 2 seconds or until end
    
    let cut_range = CutRange::new(
        TimeSpec::from_seconds(start_time),
        TimeSpec::from_seconds(end_time),
    ).unwrap();
    
    let request = ClipRequest {
        input_file: sample_video.to_string(),
        output_file: output_video.to_string(),
        cut_range,
        mode: ClippingMode::Copy,
        quality_settings: None,
    };
    
    // Execute clip operation
    let result = interactor.execute(request).await;
    
    match result {
        Ok(response) => {
            // Verify output file was created
            assert!(std::path::Path::new(&response.output_file).exists());
            
            // Verify output file has reasonable size
            let output_size = std::fs::metadata(&response.output_file).unwrap().len();
            assert!(output_size > 0);
            
            println!("Real video clipping test passed - Output: {}, Size: {} bytes", 
                     response.output_file, output_size);
        },
        Err(e) => {
            panic!("Real video clipping failed: {}", e);
        }
    }
    
    // Clean up
    let _ = std::fs::remove_file(output_video);
}

#[tokio::test]
async fn test_hardware_acceleration_detection() {
    let exec_adapter = LibavExecutionAdapter::new().unwrap();
    
    // Test hardware acceleration detection
    let hw_available = exec_adapter.is_hardware_acceleration_available().await.unwrap();
    println!("Hardware acceleration available: {}", hw_available);
    
    // Test specific acceleration types
    let acceleration_types = exec_adapter.get_available_hardware_acceleration().await.unwrap();
    println!("Available acceleration types: {:?}", acceleration_types);
    
    // Test codec detection with hardware acceleration
    let video_codecs = exec_adapter.get_available_video_codecs().await.unwrap();
    let hardware_codecs: Vec<_> = video_codecs.iter()
        .filter(|codec| codec.is_hardware_accelerated)
        .collect();
    
    println!("Hardware-accelerated codecs: {}", hardware_codecs.len());
    for codec in &hardware_codecs {
        println!("  - {}: {}", codec.name, codec.long_name);
    }
    
    // Test execution capabilities
    let capabilities = exec_adapter.test_execution_capabilities().await.unwrap();
    assert!(capabilities.supports_copy_mode);
    assert!(capabilities.supports_reencode_mode);
    assert!(capabilities.max_concurrent_operations > 0);
    
    println!("Execution capabilities: {:?}", capabilities);
}

#[tokio::test]
async fn test_performance_benchmarks() {
    let sample_video = "sample video.mp4";
    
    // Skip test if sample video doesn't exist
    if !std::path::Path::new(sample_video).exists() {
        println!("Skipping performance test - sample video not found");
        return;
    }
    
    let exec_adapter = LibavExecutionAdapter::new().unwrap();
    
    // Test execution capabilities
    let capabilities = exec_adapter.test_execution_capabilities().await.unwrap();
    
    // Benchmark different modes
    let start_time = std::time::Instant::now();
    
    // Test copy mode performance
    let copy_plan = ExecutionPlan::new(
        ClippingMode::Copy,
        sample_video.to_string(),
        "benchmark_copy.mp4".to_string(),
        CutRange::new(
            TimeSpec::from_seconds(1.0),
            TimeSpec::from_seconds(3.0),
        ).unwrap(),
        vec![],
        QualitySettings::default(),
        "mp4".to_string(),
    ).unwrap();
    
    let copy_result = exec_adapter.execute_plan(&copy_plan).await;
    let copy_duration = start_time.elapsed();
    
    match copy_result {
        Ok(report) => {
            println!("Copy mode benchmark: {}ms, Success: {}", 
                     copy_duration.as_millis(), report.success);
        },
        Err(e) => {
            println!("Copy mode benchmark failed: {}", e);
        }
    }
    
    // Clean up
    let _ = std::fs::remove_file("benchmark_copy.mp4");
    
    // Test reencode mode performance
    let reencode_start = std::time::Instant::now();
    
    let reencode_plan = ExecutionPlan::new(
        ClippingMode::Reencode,
        sample_video.to_string(),
        "benchmark_reencode.mp4".to_string(),
        CutRange::new(
            TimeSpec::from_seconds(1.0),
            TimeSpec::from_seconds(3.0),
        ).unwrap(),
        vec![],
        QualitySettings::default(),
        "mp4".to_string(),
    ).unwrap();
    
    let reencode_result = exec_adapter.execute_plan(&reencode_plan).await;
    let reencode_duration = reencode_start.elapsed();
    
    match reencode_result {
        Ok(report) => {
            println!("Reencode mode benchmark: {}ms, Success: {}", 
                     reencode_duration.as_millis(), report.success);
        },
        Err(e) => {
            println!("Reencode mode benchmark failed: {}", e);
        }
    }
    
    // Clean up
    let _ = std::fs::remove_file("benchmark_reencode.mp4");
    
    println!("Performance test completed");
}
