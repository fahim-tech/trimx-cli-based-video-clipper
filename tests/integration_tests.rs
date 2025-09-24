use trimx_cli::*;
use trimx_cli::domain::model::*;
use trimx_cli::domain::errors::*;

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
        StreamMapping::new(0, 0, true, StreamType::Video),
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
    let video_stream = VideoStreamInfo::new(
        0,
        "h264".to_string(),
        1920,
        1080,
        29.97,
        Timebase::frame_rate_30(),
    ).unwrap();
    
    let audio_stream = AudioStreamInfo::new(
        1,
        "aac".to_string(),
        48000,
        2,
        Timebase::av_time_base(),
    ).unwrap();
    
    let media_info = MediaInfo::new(
        "mp4".to_string(),
        1024000,
        vec![video_stream],
        vec![audio_stream],
        vec![],
    ).unwrap();
    
    let cut_range = CutRange::new(
        TimeSpec::from_seconds(10.0),
        TimeSpec::from_seconds(20.0),
    ).unwrap();
    
    // Test auto mode selection
    let selected_mode = ClippingModeSelector::select_mode(&media_info, &cut_range, ClippingMode::Auto).unwrap();
    assert!(matches!(selected_mode, ClippingMode::Copy | ClippingMode::Hybrid | ClippingMode::Reencode));
}

#[test]
fn test_output_report_creation() {
    let duration = TimeSpec::from_seconds(10.0);
    let processing_time = std::time::Duration::from_secs(5);
    let report = OutputReport::success(duration, 1024000, processing_time, ClippingMode::Copy);
    
    assert!(report.success);
    assert_eq!(report.duration.seconds, 10.0);
    assert_eq!(report.file_size, 1024000);
    assert_eq!(report.processing_time, processing_time);
    assert_eq!(report.mode_used, ClippingMode::Copy);
    assert!(report.warnings.is_empty());
    
    // Test failure report
    let failure_report = OutputReport::failure(ClippingMode::Reencode, "Test error".to_string());
    assert!(!failure_report.success);
    assert_eq!(failure_report.warnings.len(), 1);
    assert_eq!(failure_report.warnings[0], "Test error");
}
