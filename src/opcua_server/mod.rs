pub mod plc1;
pub mod plc2;
pub mod scada_client;

pub use plc1::start_plc1_server;
pub use plc2::start_plc2_server;
pub use scada_client::start_scada_client;
