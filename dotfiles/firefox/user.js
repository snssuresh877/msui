// =============================================================================
// Firefox Performance, Low-RAM, and Privacy Profile (Arch Linux / Hyprland)
// Hardware: AMD Renoir/Vega (Mesa VA-API) | Display: Wayland Zero-Copy DMA-BUF
// =============================================================================

// -----------------------------------------------------------------------------
// 1. Process & Memory Management (Strict Low-RAM Limits)
// -----------------------------------------------------------------------------
// Limit content processes to 2 (reduces RAM usage by 300-600 MB)
user_pref("dom.ipc.processCount", 2);

// Auto-unload background tabs when system experiences RAM pressure
user_pref("browser.tabs.unloadOnLowMemory", true);

// Limit session history entries per tab (default: 50 -> 10)
user_pref("browser.sessionhistory.max_entries", 10);

// Reduce session state write frequency to once every 5 minutes (300,000ms)
// Default is every 15,000ms (15s), which causes SSD wear and periodic micro-stutters
user_pref("browser.sessionstore.interval", 300000);
user_pref("browser.sessionstore.max_tabs_undo", 5);
user_pref("browser.sessionstore.max_windows_undo", 1);

// Limit memory & disk caches to balanced limits
user_pref("browser.cache.disk.enable", true);
user_pref("browser.cache.memory.enable", true);
user_pref("browser.cache.memory.capacity", 65536); // 64 MB cap for in-memory page cache

// -----------------------------------------------------------------------------
// 2. Hardware Video Acceleration & Native Wayland (AMD Radeon / Mesa)
// -----------------------------------------------------------------------------
// Force full WebRender hardware compositor on Wayland
user_pref("gfx.webrender.all", true);
user_pref("gfx.webrender.software", false);

// Enable hardware video decoding via VA-API / FFmpeg
user_pref("media.hardware-video-decoding.enabled", true);
user_pref("media.ffmpeg.vaapi.enabled", true);
user_pref("media.rdd-ffmpeg.enabled", true);

// Enable zero-copy DMA-BUF buffer sharing on Wayland
user_pref("widget.wayland-dmabuf-vaapi.enabled", true);
user_pref("widget.dmabuf.force-enabled", true);

// -----------------------------------------------------------------------------
// 3. Smooth Scrolling & Input Latency
// -----------------------------------------------------------------------------
user_pref("general.smoothScroll", true);
user_pref("general.smoothScroll.msdPhysics.enabled", true);
user_pref("mousewheel.min_line_scroll_amount", 20);

// -----------------------------------------------------------------------------
// 4. Network & Connection Optimization
// -----------------------------------------------------------------------------
// Enable HTTP/3 (QUIC) for lightning-fast handshake on supported servers
user_pref("network.http.http3.enable", true);

// Disable speculative background prefetching (saves CPU, RAM, and mobile data)
user_pref("network.prefetch-next", false);
user_pref("network.dns.disablePrefetch", true);
user_pref("network.http.speculative-parallel-limit", 0);

// -----------------------------------------------------------------------------
// 5. Privacy, Anti-Telemetry & Anti-Bloat
// -----------------------------------------------------------------------------
// Disable all Mozilla telemetry pings & crash reporting
user_pref("toolkit.telemetry.enabled", false);
user_pref("toolkit.telemetry.unified", false);
user_pref("toolkit.telemetry.server", "");
user_pref("browser.ping-centre.telemetry", false);
user_pref("datareporting.healthreport.uploadEnabled", false);
user_pref("app.shield.optoutstudies.enabled", false);

// Completely disable Pocket integration
user_pref("extensions.pocket.enabled", false);
user_pref("browser.newtabpage.activity-stream.section.highlights.includePocket", false);

// Disable sponsored shortcuts & top sites on new tab page
user_pref("browser.newtabpage.activity-stream.feeds.telemetry", false);
user_pref("browser.newtabpage.activity-stream.telemetry", false);
user_pref("browser.newtabpage.activity-stream.feeds.section.topstories", false);
user_pref("browser.newtabpage.activity-stream.showSponsored", false);
user_pref("browser.newtabpage.activity-stream.showSponsoredTopSites", false);

// Disable sponsored search and URL bar suggestions
user_pref("browser.urlbar.suggest.quicksuggest.sponsored", false);
user_pref("browser.urlbar.suggest.quicksuggest.nonsponsored", false);

// -----------------------------------------------------------------------------
// 6. Media & Audio/Video Autoplay Control
// -----------------------------------------------------------------------------
// Block all audio and video autoplay by default (click to play)
user_pref("media.autoplay.default", 5);
user_pref("media.autoplay.blocking_policy", 2);

// Disable annoying download animation notification
user_pref("browser.download.animateNotifications", false);
