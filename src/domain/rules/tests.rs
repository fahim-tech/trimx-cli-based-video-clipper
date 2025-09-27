// Unit tests for business rules

#[cfg(test)]
mod tests {
    use super::*;
    use crate::domain::model::*;
    use crate::domain::errors::*;
    use crate::domain::rules::*;

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
        
        MediaInfo {
            path: "test.mp4".to_string(),
            container: "mp4".to_string(),
            duration: TimeSpec::from_seconds(100.0),
            video_streams: vec![video_stream],
            audio_streams: vec![audio_stream],
            subtitle_streams: vec![],
            bit_rate: Some(1000000),
            file_size: 1000000,
            metadata: std::collections::HashMap::new(),
        }
    }

    #[test]
    fn test_clipping_mode_selector_auto_copy() {
        let media_info = create_test_media_info();
        let cut_range = CutRange::new(
            TimeSpec::from_seconds(10.0), // 10.0 seconds
            TimeSpec::from_seconds(20.0), // 20.0 seconds
        ).unwrap();
        
        let mode = ClippingModeSelector::select_mode(&media_info, &cut_range, ClippingMode::Auto).unwrap();
        assert_eq!(mode, ClippingMode::Hybrid);
    }

    #[test]
    fn test_clipping_mode_selector_force_reencode() {
        let media_info = create_test_media_info();
        let cut_range = CutRange::new(
            TimeSpec::from_seconds(10.0),
            TimeSpec::from_seconds(20.0),
        ).unwrap();
        
        let mode = ClippingModeSelector::select_mode(&media_info, &cut_range, ClippingMode::Reencode).unwrap();
        assert_eq!(mode, ClippingMode::Reencode);
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
}