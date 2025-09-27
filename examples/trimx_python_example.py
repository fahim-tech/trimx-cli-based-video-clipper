#!/usr/bin/env python3
"""
TrimX CLI Python Integration Examples

This script demonstrates advanced video processing workflows using TrimX CLI
with Python for automation, analysis, and batch processing.
"""

import os
import sys
import json
import subprocess
import tempfile
import shutil
from pathlib import Path
from typing import List, Dict, Tuple, Optional
from dataclasses import dataclass
import argparse

@dataclass
class VideoInfo:
    """Video information container"""
    path: str
    duration: float
    width: int
    height: int
    frame_rate: float
    bit_rate: int
    file_size: int
    video_streams: List[Dict]
    audio_streams: List[Dict]

@dataclass
class ClipSegment:
    """Video clip segment definition"""
    start_time: float
    end_time: float
    name: str
    description: Optional[str] = None

class TrimXProcessor:
    """TrimX CLI processor with Python integration"""
    
    def __init__(self, trimx_path: str = "trimx"):
        self.trimx_path = trimx_path
        self.temp_dir = None
        
    def __enter__(self):
        self.temp_dir = tempfile.mkdtemp(prefix="trimx_")
        return self
        
    def __exit__(self, exc_type, exc_val, exc_tb):
        if self.temp_dir and os.path.exists(self.temp_dir):
            shutil.rmtree(self.temp_dir)
    
    def run_command(self, args: List[str], capture_output: bool = True) -> subprocess.CompletedProcess:
        """Run TrimX command with error handling"""
        cmd = [self.trimx_path] + args
        
        try:
            result = subprocess.run(
                cmd,
                capture_output=capture_output,
                text=True,
                check=True
            )
            return result
        except subprocess.CalledProcessError as e:
            print(f"Error running command: {' '.join(cmd)}")
            print(f"Return code: {e.returncode}")
            print(f"Error output: {e.stderr}")
            raise
        except FileNotFoundError:
            print(f"TrimX CLI not found at: {self.trimx_path}")
            print("Please ensure TrimX is installed and in your PATH")
            raise
    
    def inspect_video(self, video_path: str) -> VideoInfo:
        """Inspect video file and return structured information"""
        print(f"Inspecting video: {video_path}")
        
        # Get basic video information
        result = self.run_command([
            "inspect", video_path,
            "--format", "json",
            "--show-streams"
        ])
        
        data = json.loads(result.stdout)
        
        # Extract video stream info
        video_streams = []
        audio_streams = []
        
        for stream in data.get("streams", []):
            if stream.get("type") == "video":
                video_streams.append(stream)
            elif stream.get("type") == "audio":
                audio_streams.append(stream)
        
        # Get primary video stream info
        primary_video = video_streams[0] if video_streams else {}
        
        return VideoInfo(
            path=video_path,
            duration=data.get("duration", 0.0),
            width=primary_video.get("width", 0),
            height=primary_video.get("height", 0),
            frame_rate=primary_video.get("frame_rate", 0.0),
            bit_rate=data.get("bit_rate", 0),
            file_size=data.get("file_size", 0),
            video_streams=video_streams,
            audio_streams=audio_streams
        )
    
    def extract_clip(self, video_path: str, start_time: float, end_time: float, 
                    output_path: str, mode: str = "auto", quality_crf: Optional[int] = None) -> bool:
        """Extract a clip from video"""
        print(f"Extracting clip: {start_time}s to {end_time}s")
        
        args = [
            "clip", video_path,
            "--start", str(start_time),
            "--end", str(end_time),
            "--mode", mode,
            "--output", output_path,
            "--overwrite", "yes"
        ]
        
        if quality_crf:
            args.extend(["--quality-crf", str(quality_crf)])
        
        try:
            self.run_command(args, capture_output=False)
            return True
        except subprocess.CalledProcessError:
            return False
    
    def verify_clip(self, clip_path: str, expected_start: float, expected_end: float, 
                   tolerance: float = 0.5) -> bool:
        """Verify extracted clip"""
        print(f"Verifying clip: {clip_path}")
        
        try:
            self.run_command([
                "verify", clip_path,
                "--expected-start", str(expected_start),
                "--expected-end", str(expected_end),
                "--tolerance", str(tolerance)
            ], capture_output=False)
            return True
        except subprocess.CalledProcessError:
            return False
    
    def batch_extract_clips(self, video_path: str, segments: List[ClipSegment], 
                          output_dir: str, mode: str = "auto") -> Dict[str, bool]:
        """Extract multiple clips in batch"""
        print(f"Batch extracting {len(segments)} clips from {video_path}")
        
        os.makedirs(output_dir, exist_ok=True)
        results = {}
        
        for i, segment in enumerate(segments):
            output_path = os.path.join(output_dir, f"{segment.name}.mp4")
            
            print(f"Processing segment {i+1}/{len(segments)}: {segment.name}")
            
            success = self.extract_clip(
                video_path,
                segment.start_time,
                segment.end_time,
                output_path,
                mode
            )
            
            if success:
                # Verify the clip
                verify_success = self.verify_clip(
                    output_path,
                    segment.start_time,
                    segment.end_time
                )
                results[segment.name] = verify_success
            else:
                results[segment.name] = False
        
        return results
    
    def analyze_keyframes(self, video_path: str) -> List[Dict]:
        """Analyze keyframes in video"""
        print(f"Analyzing keyframes: {video_path}")
        
        result = self.run_command([
            "inspect", video_path,
            "--show-keyframes",
            "--format", "json"
        ])
        
        data = json.loads(result.stdout)
        return data.get("keyframes", [])
    
    def find_optimal_cut_points(self, video_path: str, desired_start: float, 
                              desired_end: float, tolerance: float = 2.0) -> Tuple[float, float]:
        """Find optimal cut points near keyframes"""
        keyframes = self.analyze_keyframes(video_path)
        
        if not keyframes:
            print("No keyframes found, using desired times")
            return desired_start, desired_end
        
        # Find nearest keyframes
        start_keyframe = min(keyframes, key=lambda k: abs(k["timestamp"] - desired_start))
        end_keyframe = min(keyframes, key=lambda k: abs(k["timestamp"] - desired_end))
        
        # Check if keyframes are within tolerance
        if abs(start_keyframe["timestamp"] - desired_start) <= tolerance:
            optimal_start = start_keyframe["timestamp"]
        else:
            optimal_start = desired_start
            
        if abs(end_keyframe["timestamp"] - desired_end) <= tolerance:
            optimal_end = end_keyframe["timestamp"]
        else:
            optimal_end = desired_end
        
        print(f"Optimal cut points: {optimal_start:.2f}s to {optimal_end:.2f}s")
        return optimal_start, optimal_end

def create_sample_video(output_path: str, duration: int = 30) -> bool:
    """Create a sample video for testing"""
    print(f"Creating sample video: {output_path} ({duration}s)")
    
    try:
        subprocess.run([
            "ffmpeg", "-f", "lavfi",
            "-i", f"testsrc=duration={duration}:size=640x480:rate=30",
            "-f", "lavfi",
            "-i", f"sine=frequency=1000:duration={duration}",
            "-c:v", "libx264",
            "-c:a", "aac",
            "-shortest",
            "-y", output_path
        ], check=True, capture_output=True)
        return True
    except subprocess.CalledProcessError:
        return False

def demo_basic_usage():
    """Demonstrate basic TrimX usage"""
    print("=== Basic Usage Demo ===")
    
    with TrimXProcessor() as processor:
        # Create sample video
        sample_video = os.path.join(processor.temp_dir, "sample.mp4")
        if not create_sample_video(sample_video, 30):
            print("Failed to create sample video")
            return
        
        # Inspect video
        video_info = processor.inspect_video(sample_video)
        print(f"Video duration: {video_info.duration:.2f}s")
        print(f"Resolution: {video_info.width}x{video_info.height}")
        print(f"Frame rate: {video_info.frame_rate:.2f} fps")
        
        # Extract a clip
        output_path = os.path.join(processor.temp_dir, "clip.mp4")
        success = processor.extract_clip(sample_video, 5.0, 15.0, output_path)
        
        if success:
            print("Clip extraction successful")
            # Verify clip
            verify_success = processor.verify_clip(output_path, 5.0, 15.0)
            print(f"Clip verification: {'PASSED' if verify_success else 'FAILED'}")
        else:
            print("Clip extraction failed")

def demo_batch_processing():
    """Demonstrate batch processing"""
    print("\n=== Batch Processing Demo ===")
    
    with TrimXProcessor() as processor:
        # Create sample video
        sample_video = os.path.join(processor.temp_dir, "sample.mp4")
        if not create_sample_video(sample_video, 60):
            print("Failed to create sample video")
            return
        
        # Define segments to extract
        segments = [
            ClipSegment(0, 10, "intro", "Introduction segment"),
            ClipSegment(15, 25, "main1", "Main content part 1"),
            ClipSegment(30, 40, "main2", "Main content part 2"),
            ClipSegment(45, 55, "outro", "Conclusion segment")
        ]
        
        # Extract clips
        output_dir = os.path.join(processor.temp_dir, "clips")
        results = processor.batch_extract_clips(sample_video, segments, output_dir)
        
        # Report results
        print("\nBatch processing results:")
        for name, success in results.items():
            status = "SUCCESS" if success else "FAILED"
            print(f"  {name}: {status}")

def demo_quality_comparison():
    """Demonstrate quality comparison"""
    print("\n=== Quality Comparison Demo ===")
    
    with TrimXProcessor() as processor:
        # Create sample video
        sample_video = os.path.join(processor.temp_dir, "sample.mp4")
        if not create_sample_video(sample_video, 20):
            print("Failed to create sample video")
            return
        
        # Test different quality settings
        quality_settings = [
            ("ultrafast", 28, "fastest"),
            ("fast", 23, "fast"),
            ("medium", 18, "balanced"),
            ("slow", 15, "highest")
        ]
        
        output_dir = os.path.join(processor.temp_dir, "quality")
        os.makedirs(output_dir, exist_ok=True)
        
        for preset, crf, description in quality_settings:
            output_path = os.path.join(output_dir, f"quality_{preset}.mp4")
            
            print(f"Testing {description} quality (preset: {preset}, CRF: {crf})")
            
            success = processor.extract_clip(
                sample_video, 5.0, 15.0, output_path, 
                mode="reencode", quality_crf=crf
            )
            
            if success:
                file_size = os.path.getsize(output_path)
                print(f"  Output size: {file_size:,} bytes")
            else:
                print(f"  Failed to create {description} quality clip")

def demo_keyframe_analysis():
    """Demonstrate keyframe analysis and optimal cutting"""
    print("\n=== Keyframe Analysis Demo ===")
    
    with TrimXProcessor() as processor:
        # Create sample video
        sample_video = os.path.join(processor.temp_dir, "sample.mp4")
        if not create_sample_video(sample_video, 30):
            print("Failed to create sample video")
            return
        
        # Analyze keyframes
        keyframes = processor.analyze_keyframes(sample_video)
        print(f"Found {len(keyframes)} keyframes")
        
        if keyframes:
            print("Keyframe timestamps:")
            for i, kf in enumerate(keyframes[:10]):  # Show first 10
                print(f"  {i+1}: {kf['timestamp']:.2f}s")
        
        # Find optimal cut points
        desired_start = 8.5
        desired_end = 18.2
        
        optimal_start, optimal_end = processor.find_optimal_cut_points(
            sample_video, desired_start, desired_end
        )
        
        # Extract clip with optimal cut points
        output_path = os.path.join(processor.temp_dir, "optimal_clip.mp4")
        success = processor.extract_clip(
            sample_video, optimal_start, optimal_end, output_path, mode="copy"
        )
        
        if success:
            print(f"Optimal clip extracted successfully")
            print(f"Original desired: {desired_start:.2f}s to {desired_end:.2f}s")
            print(f"Actual cut: {optimal_start:.2f}s to {optimal_end:.2f}s")

def main():
    """Main function"""
    parser = argparse.ArgumentParser(description="TrimX CLI Python Integration Examples")
    parser.add_argument("--trimx-path", default="trimx", help="Path to TrimX CLI executable")
    parser.add_argument("--demo", choices=["basic", "batch", "quality", "keyframes", "all"], 
                       default="all", help="Demo to run")
    
    args = parser.parse_args()
    
    print("TrimX CLI Python Integration Examples")
    print("====================================")
    
    # Check if TrimX is available
    try:
        subprocess.run([args.trimx_path, "--version"], check=True, capture_output=True)
        print(f"Using TrimX CLI: {args.trimx_path}")
    except (subprocess.CalledProcessError, FileNotFoundError):
        print(f"Error: TrimX CLI not found at {args.trimx_path}")
        print("Please ensure TrimX is built and installed")
        sys.exit(1)
    
    # Run demos
    if args.demo in ["basic", "all"]:
        demo_basic_usage()
    
    if args.demo in ["batch", "all"]:
        demo_batch_processing()
    
    if args.demo in ["quality", "all"]:
        demo_quality_comparison()
    
    if args.demo in ["keyframes", "all"]:
        demo_keyframe_analysis()
    
    print("\nAll demos completed successfully!")

if __name__ == "__main__":
    main()
