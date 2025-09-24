use trimx_cli::*;

#[test]
fn test_time_parser() {
    let parser = TimeParser::new();
    
    // Test seconds format
    assert_eq!(parser.parse_time("90.5").unwrap(), 90.5);
    
    // Test MM:SS format
    assert_eq!(parser.parse_time("01:30").unwrap(), 90.0);
    
    // Test MM:SS.ms format
    assert_eq!(parser.parse_time("01:30.500").unwrap(), 90.5);
    
    // Test HH:MM:SS.ms format
    assert_eq!(parser.parse_time("00:01:30.500").unwrap(), 90.5);
}

#[test]
fn test_path_utils() {
    let path_utils = PathUtils::new();
    
    // Test path validation
    assert!(path_utils.validate_path("test.mov").is_ok());
    assert!(path_utils.validate_path("test<invalid>.mov").is_err());
}

#[test]
fn test_utils() {
    // Test duration formatting
    let duration = std::time::Duration::from_secs(3661);
    let formatted = Utils::format_duration(duration);
    assert_eq!(formatted, "01:01:01.000");
    
    // Test file size formatting
    assert_eq!(Utils::format_file_size(1024), "1.00 KB");
    assert_eq!(Utils::format_file_size(1048576), "1.00 MB");
}
