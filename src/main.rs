mod simulator;
mod opcua_server;
mod ws_bridge;

use simulator::plant::Plant;
use tokio::sync::{broadcast, RwLock};
use std::sync::Arc;
use std::time::Duration;

#[tokio::main]
async fn main() {
    tracing_subscriber::fmt::init();

    tracing::info!("Starting Water Plant Digital Twin with OPC UA Architecture");

    // Create broadcast channel for plant state updates
    let (tx, _rx) = broadcast::channel(100);
    let tx_ws = tx.clone();
    let tx_scada = tx.clone();

    // Create plant with Arc<RwLock> for shared access
    let plant = Arc::new(RwLock::new(Plant::new()));

    // Get references to individual devices for PLC servers
    let plant_clone = plant.clone();

    // Start WebSocket server
    let ws_server = tokio::spawn(async move {
        if let Err(e) = ws_bridge::start_ws_server(tx_ws).await {
            tracing::error!("WebSocket server error: {}", e);
        }
    });

    // Run simulation loop - updates device structs in memory
    let plant_sim = plant.clone();
    let simulation = tokio::spawn(async move {
        let mut interval = tokio::time::interval(Duration::from_millis(100));

        tracing::info!("Simulation loop started");

        loop {
            interval.tick().await;

            // Update plant simulation (dt = 0.1 seconds)
            let mut plant = plant_sim.write().await;
            plant.tick(0.1);
        }
    });

    // Create Arc<RwLock> wrappers for each device
    let boiler1 = Arc::new(RwLock::new({
        let plant = plant.read().await;
        plant.boiler_1.clone()
    }));
    let boiler2 = Arc::new(RwLock::new({
        let plant = plant.read().await;
        plant.boiler_2.clone()
    }));
    let pressure_meter1 = Arc::new(RwLock::new({
        let plant = plant.read().await;
        plant.pressure_meter_1.clone()
    }));
    let valve1 = Arc::new(RwLock::new({
        let plant = plant.read().await;
        plant.valve_1.clone()
    }));
    let flow_meter1 = Arc::new(RwLock::new({
        let plant = plant.read().await;
        plant.flow_meter_1.clone()
    }));

    // Spawn task to sync devices from plant to individual Arc<RwLock> refs
    let plant_sync = plant.clone();
    let b1_sync = boiler1.clone();
    let b2_sync = boiler2.clone();
    let pm1_sync = pressure_meter1.clone();
    let v1_sync = valve1.clone();
    let fm1_sync = flow_meter1.clone();

    tokio::spawn(async move {
        let mut interval = tokio::time::interval(Duration::from_millis(50));
        loop {
            interval.tick().await;
            let plant = plant_sync.read().await;

            *b1_sync.write().await = plant.boiler_1.clone();
            *b2_sync.write().await = plant.boiler_2.clone();
            *pm1_sync.write().await = plant.pressure_meter_1.clone();
            *v1_sync.write().await = plant.valve_1.clone();
            *fm1_sync.write().await = plant.flow_meter_1.clone();
        }
    });

    // Start PLC-1 server (boiler-1, pressure-meter-1, valve-1)
    let b1 = boiler1.clone();
    let pm1 = pressure_meter1.clone();
    let v1 = valve1.clone();
    let plc1_server = tokio::spawn(async move {
        if let Err(e) = opcua_server::start_plc1_server(b1, pm1, v1).await {
            tracing::error!("PLC-1 server error: {}", e);
        }
    });

    // Start PLC-2 server (boiler-2, flow-meter-1)
    let b2 = boiler2.clone();
    let fm1 = flow_meter1.clone();
    let plc2_server = tokio::spawn(async move {
        if let Err(e) = opcua_server::start_plc2_server(b2, fm1).await {
            tracing::error!("PLC-2 server error: {}", e);
        }
    });

    // Wait a bit for PLC servers to start
    tokio::time::sleep(Duration::from_secs(2)).await;

    // Start SCADA client (connects to both PLCs and aggregates data)
    let scada_client = tokio::spawn(async move {
        if let Err(e) = opcua_server::start_scada_client(tx_scada).await {
            tracing::error!("SCADA client error: {}", e);
        }
    });

    tracing::info!("Backend initialized:");
    tracing::info!("  - PLC-1 OPC UA Server on port 4840");
    tracing::info!("  - PLC-2 OPC UA Server on port 4841");
    tracing::info!("  - SCADA Client aggregating from both PLCs");
    tracing::info!("  - WebSocket on port 3000");

    // Wait for Ctrl+C or task completion
    tokio::select! {
        _ = tokio::signal::ctrl_c() => {
            tracing::info!("Received Ctrl+C, shutting down");
        }
        _ = ws_server => {
            tracing::info!("WebSocket server terminated");
        }
        _ = simulation => {
            tracing::info!("Simulation terminated");
        }
        _ = plc1_server => {
            tracing::info!("PLC-1 server terminated");
        }
        _ = plc2_server => {
            tracing::info!("PLC-2 server terminated");
        }
        _ = scada_client => {
            tracing::info!("SCADA client terminated");
        }
    }

    tracing::info!("Shutting down");
}
