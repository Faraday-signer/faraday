#[allow(dead_code)]
#[cfg(target_os = "linux")]
pub mod st7789;
#[allow(dead_code)]
#[cfg(target_os = "linux")]
pub mod buttons;
#[allow(dead_code)]
#[cfg(all(target_os = "linux", not(feature = "_desktop_sim")))]
pub mod pi_camera;
