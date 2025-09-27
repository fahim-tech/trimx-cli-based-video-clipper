#!/bin/bash
# trimx_example.sh - Practical example of TrimX CLI usage
# 
# This script demonstrates various TrimX CLI features with real-world examples

set -e  # Exit on any error

# Colors for output
RED='\033[0;31m'
GREEN='\033[0;32m'
YELLOW='\033[1;33m'
BLUE='\033[0;34m'
NC='\033[0m' # No Color

# Function to print colored output
print_status() {
    echo -e "${BLUE}[INFO]${NC} $1"
}

print_success() {
    echo -e "${GREEN}[SUCCESS]${NC} $1"
}

print_warning() {
    echo -e "${YELLOW}[WARNING]${NC} $1"
}

print_error() {
    echo -e "${RED}[ERROR]${NC} $1"
}

# Function to check if TrimX is available
check_trimx() {
    if ! command -v trimx &> /dev/null; then
        print_error "TrimX CLI not found. Please build and install TrimX first."
        print_status "Run: cargo build --release"
        print_status "Then: sudo cp target/release/trimx /usr/local/bin/"
        exit 1
    fi
    print_success "TrimX CLI found: $(trimx --version)"
}

# Function to create a sample video for testing
create_sample_video() {
    local output_file="$1"
    local duration="$2"
    
    print_status "Creating sample video: $output_file (${duration}s)"
    
    # Check if ffmpeg is available
    if ! command -v ffmpeg &> /dev/null; then
        print_error "FFmpeg not found. Please install FFmpeg to create sample videos."
        print_status "Install with: brew install ffmpeg (macOS) or apt install ffmpeg (Ubuntu)"
        exit 1
    fi
    
    # Create a test video with color bars and audio tone
    ffmpeg -f lavfi -i "testsrc=duration=${duration}:size=640x480:rate=30" \
           -f lavfi -i "sine=frequency=1000:duration=${duration}" \
           -c:v libx264 -c:a aac -shortest -y "$output_file" \
           >/dev/null 2>&1
    
    if [ -f "$output_file" ]; then
        print_success "Sample video created: $output_file"
    else
        print_error "Failed to create sample video"
        exit 1
    fi
}

# Function to demonstrate video inspection
demo_inspect() {
    local input_file="$1"
    
    print_status "=== Video Inspection Demo ==="
    
    # Basic inspection
    print_status "Basic video information:"
    trimx inspect "$input_file"
    
    echo ""
    print_status "Detailed stream information (JSON format):"
    trimx inspect "$input_file" --show-streams --format json | jq '.' 2>/dev/null || trimx inspect "$input_file" --show-streams --format json
    
    echo ""
    print_status "Keyframe analysis:"
    trimx inspect "$input_file" --show-keyframes
}

# Function to demonstrate video clipping
demo_clip() {
    local input_file="$1"
    local output_dir="$2"
    
    print_status "=== Video Clipping Demo ==="
    
    mkdir -p "$output_dir"
    
    # Test different clipping modes
    local modes=("copy" "reencode" "auto")
    local start_time=5
    local end_time=15
    
    for mode in "${modes[@]}"; do
        local output_file="$output_dir/clip_${mode}_${start_time}s_to_${end_time}s.mp4"
        
        print_status "Testing $mode mode..."
        print_status "Input: $input_file"
        print_status "Output: $output_file"
        print_status "Time range: ${start_time}s to ${end_time}s"
        
        # Execute clip
        if trimx clip "$input_file" \
            --start "$start_time" \
            --end "$end_time" \
            --mode "$mode" \
            --output "$output_file" \
            --overwrite yes; then
            
            # Get file size
            local file_size=$(stat -f%z "$output_file" 2>/dev/null || stat -c%s "$output_file" 2>/dev/null)
            print_success "$mode mode completed - Output size: $file_size bytes"
            
            # Verify the clip
            print_status "Verifying clip..."
            if trimx verify "$output_file" --expected-start "$start_time" --expected-end "$end_time" --tolerance 0.5; then
                print_success "Clip verification passed"
            else
                print_warning "Clip verification failed"
            fi
        else
            print_error "$mode mode failed"
        fi
        
        echo ""
    done
}

# Function to demonstrate quality settings
demo_quality() {
    local input_file="$1"
    local output_dir="$2"
    
    print_status "=== Quality Settings Demo ==="
    
    mkdir -p "$output_dir"
    
    # Test different quality presets
    local presets=("ultrafast" "fast" "medium" "slow")
    local crf_values=(28 23 18 15)
    
    for i in "${!presets[@]}"; do
        local preset="${presets[$i]}"
        local crf="${crf_values[$i]}"
        local output_file="$output_dir/quality_${preset}_crf${crf}.mp4"
        
        print_status "Testing quality preset: $preset (CRF: $crf)"
        
        if trimx clip "$input_file" \
            --start 10 \
            --end 20 \
            --mode reencode \
            --quality-preset "$preset" \
            --quality-crf "$crf" \
            --output "$output_file" \
            --overwrite yes; then
            
            local file_size=$(stat -f%z "$output_file" 2>/dev/null || stat -c%s "$output_file" 2>/dev/null)
            print_success "$preset preset completed - Output size: $file_size bytes"
        else
            print_error "$preset preset failed"
        fi
        
        echo ""
    done
}

# Function to demonstrate batch processing
demo_batch() {
    local input_file="$1"
    local output_dir="$2"
    
    print_status "=== Batch Processing Demo ==="
    
    mkdir -p "$output_dir"
    
    # Define multiple clips to extract
    local clips=(
        "0:5:0:10"    # 0-10 seconds
        "0:15:0:20"   # 15-20 seconds
        "0:25:0:30"   # 25-30 seconds
    )
    
    for i in "${!clips[@]}"; do
        local clip_info="${clips[$i]}"
        IFS=':' read -r start_min start_sec end_min end_sec <<< "$clip_info"
        local start_total=$((start_min * 60 + start_sec))
        local end_total=$((end_min * 60 + end_sec))
        
        local output_file="$output_dir/batch_clip_$((i+1)).mp4"
        
        print_status "Creating batch clip $((i+1)): ${start_total}s to ${end_total}s"
        
        if trimx clip "$input_file" \
            --start "$start_total" \
            --end "$end_total" \
            --mode auto \
            --output "$output_file" \
            --overwrite yes; then
            
            print_success "Batch clip $((i+1)) completed"
        else
            print_error "Batch clip $((i+1)) failed"
        fi
    done
}

# Function to demonstrate error handling
demo_error_handling() {
    print_status "=== Error Handling Demo ==="
    
    # Test with non-existent file
    print_status "Testing with non-existent file..."
    if trimx clip "non_existent_file.mp4" --start 0 --end 10 --output "test.mp4" 2>/dev/null; then
        print_warning "Expected error did not occur"
    else
        print_success "Correctly handled non-existent file"
    fi
    
    # Test with invalid time range
    print_status "Testing with invalid time range..."
    if trimx clip "sample_video.mp4" --start 100 --end 10 --output "test.mp4" 2>/dev/null; then
        print_warning "Expected error did not occur"
    else
        print_success "Correctly handled invalid time range"
    fi
}

# Function to show performance metrics
show_performance() {
    local output_dir="$1"
    
    print_status "=== Performance Summary ==="
    
    if [ -d "$output_dir" ]; then
        local total_files=$(find "$output_dir" -name "*.mp4" | wc -l)
        local total_size=$(find "$output_dir" -name "*.mp4" -exec stat -f%z {} \; 2>/dev/null | awk '{sum+=$1} END {print sum}' || find "$output_dir" -name "*.mp4" -exec stat -c%s {} \; | awk '{sum+=$1} END {print sum}')
        
        print_status "Total output files: $total_files"
        print_status "Total output size: $total_size bytes"
        
        if [ "$total_size" -gt 0 ]; then
            local size_mb=$((total_size / 1024 / 1024))
            print_status "Total output size: ${size_mb} MB"
        fi
    fi
}

# Main function
main() {
    print_status "TrimX CLI Example Script"
    print_status "========================"
    
    # Check prerequisites
    check_trimx
    
    # Create sample video
    local sample_video="sample_video.mp4"
    if [ ! -f "$sample_video" ]; then
        create_sample_video "$sample_video" 30
    else
        print_status "Using existing sample video: $sample_video"
    fi
    
    # Create output directory
    local output_dir="trimx_examples_output"
    mkdir -p "$output_dir"
    
    # Run demonstrations
    demo_inspect "$sample_video"
    echo ""
    
    demo_clip "$sample_video" "$output_dir/clips"
    echo ""
    
    demo_quality "$sample_video" "$output_dir/quality"
    echo ""
    
    demo_batch "$sample_video" "$output_dir/batch"
    echo ""
    
    demo_error_handling
    echo ""
    
    show_performance "$output_dir"
    
    print_success "All demonstrations completed!"
    print_status "Check the '$output_dir' directory for output files"
    print_status "Run 'ls -la $output_dir' to see all generated files"
}

# Run main function
main "$@"
