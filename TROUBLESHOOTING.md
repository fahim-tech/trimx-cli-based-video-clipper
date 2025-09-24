# TrimX CLI Video Clipper - Troubleshooting Guide

## Common Issues and Solutions

### Installation Issues

#### FFmpeg Not Found
**Error**: `Failed to initialize FFmpeg`

**Solution**:
1. Install FFmpeg development libraries
2. Ensure FFmpeg is in your PATH
3. For Windows, download FFmpeg from https://ffmpeg.org/download.html

#### Rust Toolchain Issues
**Error**: `rustc: command not found`

**Solution**:
1. Install Rust: https://rustup.rs/
2. Update toolchain: `rustup update`
3. Verify installation: `rustc --version`

### Runtime Issues

#### File Not Found
**Error**: `Input file not found: path/to/file`

**Solutions**:
- Check file path is correct
- Ensure file exists and is readable
- Use absolute paths if relative paths fail
- Check file permissions

#### Invalid Time Format
**Error**: `Invalid time format: 1:30`

**Solutions**:
- Use proper format: `MM:SS.ms` or `HH:MM:SS.ms`
- Examples: `01:30.500`, `00:01:30.500`
- Or use seconds: `90.5`

#### Permission Denied
**Error**: `Permission denied` when writing output

**Solutions**:
- Check write permissions for output directory
- Run as administrator if needed
- Ensure output path is not in use by another program
- Try different output location

#### Memory Issues
**Error**: `Out of memory` during processing

**Solutions**:
- Use `--mode copy` for large files
- Process smaller segments
- Close other applications
- Increase virtual memory

### Performance Issues

#### Slow Processing
**Symptoms**: Clipping takes much longer than expected

**Solutions**:
- Use `--mode copy` for lossless, fast clipping
- Use `--preset fast` for faster encoding
- Ensure input file is not fragmented
- Use SSD storage for better I/O performance

#### High CPU Usage
**Symptoms**: CPU usage stays at 100%

**Solutions**:
- Use `--preset medium` or `--preset fast`
- Limit concurrent operations
- Use hardware acceleration if available

### Output Issues

#### Corrupted Output
**Symptoms**: Output file won't play or is corrupted

**Solutions**:
- Verify input file is not corrupted
- Try `--mode reencode` for exact cuts
- Check available disk space
- Use different output container format

#### Wrong Duration
**Symptoms**: Output duration doesn't match expected range

**Solutions**:
- Use `--mode reencode` for precise cuts
- Check if start/end times are on keyframes
- Verify input file duration
- Use `inspect` command to analyze file

#### Missing Streams
**Symptoms**: Audio or video missing in output

**Solutions**:
- Check if `--no-audio` or `--no-subs` flags were used
- Verify input file has the expected streams
- Use `inspect` command to see available streams

### Debugging

#### Enable Debug Logging
```bash
clipper.exe --log-level debug clip --in file.mov --start 00:01:00 --end 00:02:00
```

#### JSON Output for Automation
```bash
clipper.exe inspect --in file.mov --json
```

#### Verify Output
```bash
clipper.exe verify --in output.mov --start 00:01:00 --end 00:02:00
```

### Getting Help

#### Check Logs
Look for detailed error messages in the console output.

#### Report Issues
When reporting issues, include:
- TrimX version: `clipper.exe --version`
- Input file format and size
- Exact command used
- Full error message
- Operating system version

#### Community Support
- GitHub Issues: [Report bugs](https://github.com/yourusername/trimx-cli-based-video-clipper/issues)
- GitHub Discussions: [Ask questions](https://github.com/yourusername/trimx-cli-based-video-clipper/discussions)

### Advanced Troubleshooting

#### FFmpeg Debugging
If FFmpeg errors occur, try:
1. Test with FFmpeg directly: `ffmpeg -i input.mov -t 10 -c copy output.mov`
2. Check FFmpeg version compatibility
3. Verify codec support

#### File System Issues
For Windows long path issues:
1. Enable long path support in Windows
2. Use `\\?\` prefix for very long paths
3. Ensure file system supports large files

#### Hardware Acceleration
If hardware acceleration fails:
1. Update graphics drivers
2. Check codec support
3. Fall back to software encoding
4. Use `--preset medium` for compatibility
