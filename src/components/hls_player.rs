#![allow(non_snake_case)]

use topcoat::asset::asset;
use topcoat::{
    Result,
    view::{component, view},
};

/// hls.js 运行库（构建期打包进 bundle，不依赖 CDN）
const HLS_JS: topcoat::asset::Asset = asset!("public/js/hls.min.js");

/// HLS 视频播放器：单视频场景（上传结果页、arc 详情）。
/// Chrome/Firefox 经 hls.js 播放，Safari 原生直播
#[component]
pub async fn HlsPlayer(src: String) -> Result {
    view! {
        <video
            id="hls-player"
            controls=""
            playsinline=""
            class="w-full h-full"
            src=(src)
        ></video>
        <script src=(HLS_JS)></script>
        <script>
            "(function(){var v=document.getElementById('hls-player');if(!v)return;if(v.canPlayType('application/vnd.apple.mpegurl'))return;if(window.Hls&&Hls.isSupported()){var h=new Hls();h.loadSource(v.src);h.attachMedia(v)}})();"
        </script>
    }
}

/// 全局 HLS 接管：扫描页面上 src 以 .m3u8 结尾的 video
/// （Markdown 描述内嵌的手写 <video> 亦可用），Chrome/Firefox 交 hls.js
#[component]
pub async fn HlsScan() -> Result {
    view! {
        <script src=(HLS_JS)></script>
        <script>
            "(function(){var vs=document.querySelectorAll('video');for(var i=0;i<vs.length;i++){var v=vs[i];var s=v.src||'';if(s.slice(-5)!=='.m3u8')continue;if(v.canPlayType('application/vnd.apple.mpegurl'))continue;if(window.Hls&&Hls.isSupported()){var h=new Hls();h.loadSource(v.src);h.attachMedia(v)}}})();"
        </script>
    }
}
