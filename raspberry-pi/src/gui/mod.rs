pub use faraday_core::gui::app;
pub use faraday_core::gui::colors;
pub use faraday_core::gui::screens;

#[cfg(feature = "simulator_no_cam")]
pub mod file_camera;
#[cfg(feature = "_desktop_sim")]
pub mod framebuffer;
#[cfg(feature = "simulator")]
pub mod sim_camera;
