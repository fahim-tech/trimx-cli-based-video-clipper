// Unit tests for business rules

#[cfg(test)]
mod tests {
    use super::*;
    use crate::domain::model::*;
    use crate::domain::errors::*;

    fn create_test_media_info() -> MediaInfo {
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
        
        MediaInfo::new(
            "mp4".to_string(),
            1000000,
            vec![video_stream],
            vec![audio_stream],
            vec![],
        ).unwrap()
    }

    #[test]
    fn test_clipping_mode_selector_auto_copy() {
        let media_info = create_test_media_info();
        let cut_range = CutRange::new(
            TimeSpec::from_seconds(10.0),
            TimeSpec::from_seconds(20.0),
        ).unwrap();
        
        let mode = ClippingModeSelector::select_mode(&media_info, &cut_range, ClippingMode::Auto).unwrap();
        assert_eq!(mode, ClippingMode::Copy);
    }

    #[test]
    fn test_clipping_mode_selector_requested_mode() {
        let media_info = create_test_media_info();
        let cut_range = CutRange::new(
            TimeSpec::from_seconds(10.0),
            TimeSpec::from_seconds(20.0),
        ).unwrap();
        
        let mode = ClippingModeSelector::select_mode(&media_info, &cut_range, ClippingMode::Reencode).unwrap();
        assert_eq!(mode, ClippingMode::Reencode);
    }

    #[test]
    fn test_keyframe_analyzer_proximity() {
        let video_stream = VideoStreamInfo::new(
            0,
            "h264".to_string(),
            1920,
            1080,
            30.0,
            Timebase::frame_rate_30(),
        ).unwrap();
        
        let cut_range = CutRange::new(
            TimeSpec::from_seconds(1.0),
            TimeSpec::from_seconds(2.0),
        ).unwrap();
        
        let proximity = KeyframeAnalyzer::analyze_keyframe_proximity(&video_stream, &cut_range);
        
        assert!(proximity.is_copy_viable);
        assert!(proximity.start_distance < 0.5);
        assert!(proximity.end_distance < 0.5);
    }

    #[test]
    fn test_quality_settings_selector_copy_mode() {
        let media_info = create_test_media_info();
        let settings = QualitySettingsSelector::select_quality_settings(
            &ClippingMode::Copy,
            &media_info,
            false,
        );
        
        // Copy mode should use default settings
        assert_eq!(settings.preset, "medium");
        assert_eq!(settings.crf, Some(18));
    }

    #[test]
    fn test_quality_settings_selector_reencode_mode() {
        let media_info = create_test_media_info();
        let settings = QualitySettingsSelector::select_quality_settings(
            &ClippingMode::Reencode,
            &media_info,
            false,
        );
        
        // Re-encode mode should have optimized settings
        assert_eq!(settings.preset, "medium");
        assert_eq!(settings.hardware_acceleration, false);
    }

    #[test]
    fn test_quality_settings_selector_hybrid_mode() {
        let media_info = create_test_media_info();
        let settings = QualitySettingsSelector::select_quality_settings(
            &ClippingMode::Hybrid,
            &media_info,
            true,
        );
        
        // Hybrid mode should prioritize speed
        assert_eq!(settings.preset, "fast");
        assert_eq!(settings.hardware_acceleration, true);
    }

    #[test]
    fn test_stream_mapper_copy_mode() {
        let media_info = create_test_media_info();
        let mappings = StreamMapper::create_stream_mappings(&media_info, &ClippingMode::Copy).unwrap();
        
        assert_eq!(mappings.len(), 2); // 1 video + 1 audio
        assert!(mappings.iter().all(|m| m.copy));
    }

    #[test]
    fn test_stream_mapper_reencode_mode() {
        let media_info = create_test_media_info();
        let mappings = StreamMapper::create_stream_mappings(&media_info, &ClippingMode::Reencode).unwrap();
        
        assert_eq!(mappings.len(), 2); // 1 video + 1 audio
        assert!(mappings.iter().all(|m| !m.copy));
    }

    #[test]
    fn test_stream_mapper_hybrid_mode() {
        let media_info = create_test_media_info();
        let mappings = StreamMapper::create_stream_mappings(&media_info, &ClippingMode::Hybrid).unwrap();
        
        assert_eq!(mappings.len(), 2); // 1 video + 1 audio
        assert!(mappings.iter().all(|m| m.copy));
    }

    #[test]
    fn test_stream_copy_support_video() {
        let h264_stream = VideoStreamInfo::new(
            0,
            "h264".to_string(),
            1920,
            1080,
            30.0,
            Timebase::frame_rate_30(),
        ).unwrap();
        assert!(h264_stream.supports_copy());
        
        let unsupported_stream = VideoStreamInfo::new(
            0,
            "unsupported".to_string(),
            1920,
            1080,
            30.0,
            Timebase::frame_rate_30(),
        ).unwrap();
        assert!(!unsupported_stream.supports_copy());
    }

    #[test]
    fn test_stream_copy_support_audio() {
        let aac_stream = AudioStreamInfo::new(
            0,
            "aac".to_string(),
            48000,
            2,
            Timebase::av_time_base(),
        ).unwrap();
        assert!(aac_stream.supports_copy());
        
        let unsupported_stream = AudioStreamInfo::new(
            0,
            "unsupported".to_string(),
            48000,
            2,
            Timebase::av_time_base(),
        ).unwrap();
        assert!(!unsupported_stream.supports_copy());
    }

    #[test]
    fn test_stream_copy_support_subtitle() {
        let srt_stream = SubtitleStreamInfo::new(0, "srt".to_string());
        assert!(srt_stream.supports_copy());
        
        let unsupported_stream = SubtitleStreamInfo::new(0, "unsupported".to_string());
        assert!(!unsupported_stream.supports_copy());
    }

    #[test]
    fn test_output_validator_success() {
        let expected_duration = TimeSpec::from_seconds(10.0);
        let actual_duration = TimeSpec::from_seconds(10.05); // 50ms difference
        let tolerance_ms = 100;
        
        let report = OutputReport::success(
            actual_duration,
            1024000,
            std::time::Duration::from_secs(5),
            ClippingMode::Copy,
        );
        
        let result = OutputValidator::validate_output(&report, &expected_duration, tolerance_ms);
        
        assert!(result.overall_valid);
        assert!(result.duration_valid);
        assert!(result.success_valid);
        assert!(result.size_valid);
        assert_eq!(result.duration_difference_ms, 50);
    }

    #[test]
    fn test_output_validator_duration_failure() {
        let expected_duration = TimeSpec::from_seconds(10.0);
        let actual_duration = TimeSpec::from_seconds(11.0); // 1 second difference
        let tolerance_ms = 100; // 100ms tolerance
        
        let report = OutputReport::success(
            actual_duration,
            1024000,
            std::time::Duration::from_secs(5),
            ClippingMode::Copy,
        );
        
        let result = OutputValidator::validate_output(&report, &expected_duration, tolerance_ms);
        
        assert!(!result.overall_valid);
        assert!(!result.duration_valid);
        assert!(result.success_valid);
        assert!(result.size_valid);
        assert_eq!(result.duration_difference_ms, 1000);
    }

    #[test]
    fn test_output_validator_failure_report() {
        let expected_duration = TimeSpec::from_seconds(10.0);
        let tolerance_ms = 100;
        
        let report = OutputReport::failure(
            ClippingMode::Copy,
            "Processing failed".to_string(),
        );
        
        let result = OutputValidator::validate_output(&report, &expected_duration, tolerance_ms);
        
        assert!(!result.overall_valid);
        assert!(result.duration_valid); // Duration check passes (0.0 vs expected)
        assert!(!result.success_valid);
        assert!(!result.size_valid);
    }

    #[test]
    fn test_output_validator_empty_file() {
        let expected_duration = TimeSpec::from_seconds(10.0);
        let tolerance_ms = 100;
        
        let report = OutputReport {
            success: true,
            duration: TimeSpec::from_seconds(10.0),
            file_size: 0, // Empty file
            processing_time: std::time::Duration::from_secs(5),
            mode_used: ClippingMode::Copy,
            warnings: Vec::new(),
            first_pts: None,
            last_pts: None,
        };
        
        let result = OutputValidator::validate_output(&report, &expected_duration, tolerance_ms);
        
        assert!(!result.overall_valid);
        assert!(result.duration_valid);
        assert!(result.success_valid);
        assert!(!result.size_valid);
    }
}
