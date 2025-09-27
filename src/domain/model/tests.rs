// Unit tests for domain models

#[cfg(test)]
mod tests {
    use super::*;
    use crate::domain::errors::*;
    use crate::domain::model::*;

    #[test]
    fn test_time_spec_from_seconds() {
        let time = TimeSpec::from_seconds(3661.5);
        assert_eq!(time.seconds, 3661.5);
    }

    #[test]
    fn test_time_spec_from_components() {
        let time = TimeSpec::from_components(1, 2, 3, 500);
        assert_eq!(time.seconds, 3723.5);
    }

    #[test]
    fn test_time_spec_parse_seconds() {
        let time = TimeSpec::parse("123.456").unwrap();
        assert_eq!(time.seconds, 123.456);
    }

    #[test]
    fn test_time_spec_parse_mm_ss() {
        let time = TimeSpec::parse("01:30.5").unwrap();
        assert_eq!(time.seconds, 90.5);
    }

    #[test]
    fn test_time_spec_parse_hh_mm_ss() {
        let time = TimeSpec::parse("01:02:03.456").unwrap();
        assert_eq!(time.seconds, 3723.456);
    }

    #[test]
    fn test_time_spec_parse_invalid() {
        assert!(TimeSpec::parse("invalid").is_err());
        assert!(TimeSpec::parse("25:00").is_err()); // Invalid hours
        assert!(TimeSpec::parse("00:60").is_err()); // Invalid minutes
        assert!(TimeSpec::parse("-10").is_err()); // Negative time
    }

    #[test]
    fn test_time_spec_display() {
        let time = TimeSpec::from_components(1, 2, 3, 456);
        assert_eq!(format!("{}", time), "01:02:03.456");
        
        let time_no_hours = TimeSpec::from_components(0, 2, 3, 456);
        assert_eq!(format!("{}", time_no_hours), "02:03.456");
    }

    #[test]
    fn test_timebase_creation() {
        let timebase = Timebase::new(1, 30).unwrap();
        assert_eq!(timebase.num, 1);
        assert_eq!(timebase.den, 30);
        assert_eq!(timebase.to_seconds(), 1.0 / 30.0);
    }

    #[test]
    fn test_timebase_invalid() {
        assert!(Timebase::new(1, 0).is_err());
    }

    #[test]
    fn test_timebase_pts_conversion() {
        let timebase = Timebase::new(1, 30).unwrap();
        let pts = 150;
        let seconds = timebase.pts_to_seconds(pts);
        assert_eq!(seconds, 5.0);
        
        let back_to_pts = timebase.seconds_to_pts(seconds);
        assert_eq!(back_to_pts, pts);
    }

    #[test]
    fn test_video_stream_info_creation() {
        let timebase = Timebase::new(1, 30).unwrap();
        let stream = VideoStreamInfo::new(0, "h264".to_string(), 1920, 1080, 29.97, timebase).unwrap();
        
        assert_eq!(stream.index, 0);
        assert_eq!(stream.codec, "h264");
        assert_eq!(stream.width, 1920);
        assert_eq!(stream.height, 1080);
        assert_eq!(stream.frame_rate, 29.97);
        assert_eq!(stream.aspect_ratio(), 16.0 / 9.0);
        assert!(stream.supports_copy());
    }

    #[test]
    fn test_video_stream_info_invalid() {
        let timebase = Timebase::new(1, 30).unwrap();
        assert!(VideoStreamInfo::new(0, "h264".to_string(), 0, 1080, 29.97, timebase.clone()).is_err());
        assert!(VideoStreamInfo::new(0, "h264".to_string(), 1920, 0, 29.97, timebase.clone()).is_err());
        assert!(VideoStreamInfo::new(0, "h264".to_string(), 1920, 1080, 0.0, timebase).is_err());
    }

    #[test]
    fn test_audio_stream_info_creation() {
        let timebase = Timebase::av_time_base();
        let stream = AudioStreamInfo::new(0, "aac".to_string(), 48000, 2, timebase).unwrap();
        
        assert_eq!(stream.index, 0);
        assert_eq!(stream.codec, "aac");
        assert_eq!(stream.sample_rate, 48000);
        assert_eq!(stream.channels, 2);
        assert!(stream.supports_copy());
    }

    #[test]
    fn test_audio_stream_info_invalid() {
        let timebase = Timebase::av_time_base();
        assert!(AudioStreamInfo::new(0, "aac".to_string(), 0, 2, timebase.clone()).is_err());
        assert!(AudioStreamInfo::new(0, "aac".to_string(), 48000, 0, timebase).is_err());
    }

    #[test]
    fn test_cut_range_creation() {
        let start = TimeSpec::from_seconds(10.0);
        let end = TimeSpec::from_seconds(20.0);
        let range = CutRange::new(start, end).unwrap();
        
        assert_eq!(range.start.seconds, 10.0);
        assert_eq!(range.end.seconds, 20.0);
        assert_eq!(range.duration().seconds, 10.0);
    }

    #[test]
    fn test_cut_range_invalid() {
        let start = TimeSpec::from_seconds(10.0);
        let end = TimeSpec::from_seconds(5.0); // End before start
        assert!(CutRange::new(start, end).is_err());
        
        let start = TimeSpec::from_seconds(-1.0); // Negative start
        let end = TimeSpec::from_seconds(10.0);
        assert!(CutRange::new(start, end).is_err());
    }

    #[test]
    fn test_cut_range_validation() {
        let start = TimeSpec::from_seconds(10.0);
        let end = TimeSpec::from_seconds(20.0);
        let range = CutRange::new(start, end).unwrap();
        
        let media_duration = TimeSpec::from_seconds(30.0);
        assert!(range.validate_against_duration(&media_duration).is_ok());
        
        let media_duration = TimeSpec::from_seconds(15.0); // Too short
        assert!(range.validate_against_duration(&media_duration).is_err());
    }

    #[test]
    fn test_cut_range_keyframe_alignment() {
        let start = TimeSpec::from_seconds(1.0);
        let end = TimeSpec::from_seconds(2.0);
        let _range = CutRange::new(start, end).unwrap();
        
        let frame_duration = 1.0 / 30.0; // 30 fps
        let tolerance = frame_duration * 0.1;
        
        // Test aligned times
        let aligned_start = TimeSpec::from_seconds(1.0);
        let aligned_end = TimeSpec::from_seconds(2.0);
        let aligned_range = CutRange::new(aligned_start, aligned_end).unwrap();
        assert!(aligned_range.is_keyframe_aligned(frame_duration, tolerance));
        
        // Test misaligned times
        let misaligned_start = TimeSpec::from_seconds(1.05);
        let misaligned_end = TimeSpec::from_seconds(2.05);
        let misaligned_range = CutRange::new(misaligned_start, misaligned_end).unwrap();
        assert!(!misaligned_range.is_keyframe_aligned(frame_duration, tolerance));
    }

    #[test]
    fn test_clipping_mode_parse() {
        assert_eq!(ClippingMode::parse("auto").unwrap(), ClippingMode::Auto);
        assert_eq!(ClippingMode::parse("copy").unwrap(), ClippingMode::Copy);
        assert_eq!(ClippingMode::parse("reencode").unwrap(), ClippingMode::Reencode);
        assert_eq!(ClippingMode::parse("hybrid").unwrap(), ClippingMode::Hybrid);
        assert_eq!(ClippingMode::parse("AUTO").unwrap(), ClippingMode::Auto); // Case insensitive
        
        assert!(ClippingMode::parse("invalid").is_err());
    }

    #[test]
    fn test_quality_settings_creation() {
        let settings = QualitySettings::new(
            "medium".to_string(),
            Some(18),
            Some(5000000),
            false,
        ).unwrap();
        
        assert_eq!(settings.preset, "medium");
        assert_eq!(settings.crf, Some(18));
        assert_eq!(settings.bitrate, Some(5000000));
        assert_eq!(settings.hardware_acceleration, false);
    }

    #[test]
    fn test_quality_settings_invalid_crf() {
        assert!(QualitySettings::new(
            "medium".to_string(),
            Some(60), // Invalid CRF
            None,
            false,
        ).is_err());
    }

    #[test]
    fn test_quality_settings_default() {
        let settings = QualitySettings::default();
        assert_eq!(settings.preset, "medium");
        assert_eq!(settings.crf, Some(18));
        assert_eq!(settings.hardware_acceleration, false);
    }

    #[test]
    fn test_output_report_success() {
        let duration = TimeSpec::from_seconds(10.0);
        let processing_time = std::time::Duration::from_secs(5);
        let report = OutputReport::success(duration, 1024000, processing_time, ClippingMode::Copy);
        
        assert!(report.success);
        assert_eq!(report.duration.seconds, 10.0);
        assert_eq!(report.file_size, 1024000);
        assert_eq!(report.processing_time, processing_time);
        assert_eq!(report.mode_used, ClippingMode::Copy);
        assert!(report.warnings.is_empty());
    }

    #[test]
    fn test_output_report_failure() {
        let report = OutputReport::failure(ClippingMode::Reencode, "Test error".to_string());
        
        assert!(!report.success);
        assert_eq!(report.duration.seconds, 0.0);
        assert_eq!(report.file_size, 0);
        assert_eq!(report.mode_used, ClippingMode::Reencode);
        assert_eq!(report.warnings.len(), 1);
        assert_eq!(report.warnings[0], "Test error");
    }

    #[test]
    fn test_stream_mapping_creation() {
        let mapping = StreamMapping::new(0, 0, true, StreamType::Video);
        
        assert_eq!(mapping.input_index, 0);
        assert_eq!(mapping.output_index, 0);
        assert_eq!(mapping.copy, true);
        assert_eq!(mapping.stream_type, StreamType::Video);
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
    fn test_execution_plan_invalid() {
        let cut_range = CutRange::new(
            TimeSpec::from_seconds(10.0),
            TimeSpec::from_seconds(20.0),
        ).unwrap();
        
        let quality_settings = QualitySettings::default();
        
        // Empty input file
        assert!(ExecutionPlan::new(
            ClippingMode::Copy,
            "".to_string(),
            "output.mp4".to_string(),
            cut_range.clone(),
            vec![],
            quality_settings.clone(),
            "mp4".to_string(),
        ).is_err());
        
        // Empty output file
        assert!(ExecutionPlan::new(
            ClippingMode::Copy,
            "input.mp4".to_string(),
            "".to_string(),
            cut_range.clone(),
            vec![],
            quality_settings.clone(),
            "mp4".to_string(),
        ).is_err());
        
        // Empty streams
        assert!(ExecutionPlan::new(
            ClippingMode::Copy,
            "input.mp4".to_string(),
            "output.mp4".to_string(),
            cut_range,
            vec![],
            quality_settings,
            "mp4".to_string(),
        ).is_err());
    }
}
