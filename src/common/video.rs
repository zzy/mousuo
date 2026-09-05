//! 媒体处理 — ffmpeg 校验与 HLS 转码

use std::path::Path;
use std::process::Command;

/// ffprobe 校验视频，失败返回错误信息
pub fn probe(path: &Path) -> Result<(), String> {
    let output = Command::new("ffprobe")
        .args(["-v", "quiet", "-print_format", "json", "-show_streams", &path.to_string_lossy()])
        .output()
        .map_err(|e| format!("ffprobe 执行失败: {e}"))?;

    if !output.status.success() {
        return Err(String::from_utf8_lossy(&output.stderr).to_string());
    }

    let json: serde_json::Value =
        serde_json::from_slice(&output.stdout).map_err(|e| format!("ffprobe 解析失败: {e}"))?;

    json["streams"]
        .as_array()
        .and_then(|s| s.iter().find(|s| s["codec_type"] == "video"))
        .ok_or_else(|| "未找到视频轨道".to_string())?;

    Ok(())
}

/// ffmpeg 转码为 HLS
pub fn transcode_to_hls(input: &Path, output_dir: &Path) -> Result<(), String> {
    std::fs::create_dir_all(output_dir).map_err(|e| format!("创建输出目录失败: {e}"))?;

    let output = Command::new("ffmpeg")
        .args([
            "-i", &input.to_string_lossy(),
            "-c:v", "libx264", "-c:a", "aac",
            "-hls_time", "6", "-hls_list_size", "0",
            "-hls_segment_filename", &output_dir.join("segment_%03d.ts").to_string_lossy(),
            &output_dir.join("playlist.m3u8").to_string_lossy(),
        ])
        .output()
        .map_err(|e| format!("ffmpeg 执行失败: {e}"))?;

    if !output.status.success() {
        return Err(String::from_utf8_lossy(&output.stderr).to_string());
    }
    Ok(())
}
