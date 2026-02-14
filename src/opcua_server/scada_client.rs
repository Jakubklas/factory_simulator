use opcua::client::prelude::*;
use tokio::sync::broadcast;
use std::sync::Arc;
use opcua::sync::RwLock;
use std::str::FromStr;
use crate::simulator::plant::{PlantState, DeviceState};
use crate::simulator::devices::{Boiler, BoilerStatus, PressureMeter, MeterStatus, FlowMeter, FlowMeterStatus, Valve, ValveMode, ValveStatus};
use std::collections::HashMap;

pub async fn start_scada_client(
    tx: broadcast::Sender<PlantState>,
) -> Result<(), Box<dyn std::error::Error>> {
    tracing::info!("Starting SCADA Client - connecting to PLC-1 and PLC-2");

    // Create client for PLC-1
    let mut client1 = ClientBuilder::new()
        .application_name("SCADA")
        .application_uri("urn:SCADA")
        .create_sample_keypair(true)
        .trust_server_certs(true)
        .session_retry_limit(3)
        .client()
        .unwrap();

    // Create client for PLC-2
    let mut client2 = ClientBuilder::new()
        .application_name("SCADA")
        .application_uri("urn:SCADA")
        .create_sample_keypair(true)
        .trust_server_certs(true)
        .session_retry_limit(3)
        .client()
        .unwrap();

    // Connect to PLC-1
    let session1 = client1.connect_to_endpoint(
        (
            "opc.tcp://localhost:4840",
            SecurityPolicy::None.to_str(),
            MessageSecurityMode::None,
            UserTokenPolicy::anonymous(),
        ),
        IdentityToken::Anonymous,
    )?;

    // Connect to PLC-2
    let session2 = client2.connect_to_endpoint(
        (
            "opc.tcp://localhost:4841",
            SecurityPolicy::None.to_str(),
            MessageSecurityMode::None,
            UserTokenPolicy::anonymous(),
        ),
        IdentityToken::Anonymous,
    )?;

    tracing::info!("SCADA connected to both PLCs");

    // Polling loop to read values from both PLCs
    tokio::spawn(async move {
        let mut interval = tokio::time::interval(tokio::time::Duration::from_millis(100));

        loop {
            interval.tick().await;

            // Read from PLC-1
            let plc1_data = read_plc1_data(&session1).await;

            // Read from PLC-2
            let plc2_data = read_plc2_data(&session2).await;

            // Aggregate data
            if let (Ok(mut devices), Ok(plc2_devices)) = (plc1_data, plc2_data) {
                devices.extend(plc2_devices);

                let plant_state = PlantState { devices };
                let _ = tx.send(plant_state);
            }
        }
    });

    Ok(())
}

async fn read_plc1_data(session: &Arc<RwLock<Session>>) -> Result<HashMap<String, DeviceState>, Box<dyn std::error::Error>> {
    let session = session.read();
    let mut devices = HashMap::new();

    // Read Boiler-1 data
    let boiler1_temp = read_node_value(&session, "ns=2;s=PLC1.Boiler1.Temperature").await.unwrap_or(0.0);
    let boiler1_target_temp = read_node_value(&session, "ns=2;s=PLC1.Boiler1.TargetTemperature").await.unwrap_or(0.0);
    let boiler1_pressure = read_node_value(&session, "ns=2;s=PLC1.Boiler1.Pressure").await.unwrap_or(0.0);
    let boiler1_status_str = read_node_string(&session, "ns=2;s=PLC1.Boiler1.Status").await.unwrap_or_else(|_| "Off".to_string());

    let boiler1 = Boiler {
        id: "boiler-1".to_string(),
        temperature: boiler1_temp,
        target_temperature: boiler1_target_temp,
        pressure: boiler1_pressure,
        status: parse_boiler_status(&boiler1_status_str),
    };
    devices.insert("boiler-1".to_string(), DeviceState::Boiler(boiler1));

    // Read Pressure Meter-1 data
    let pm1_pressure = read_node_value(&session, "ns=2;s=PLC1.PressureMeter1.Pressure").await.unwrap_or(0.0);
    let pm1_status_str = read_node_string(&session, "ns=2;s=PLC1.PressureMeter1.Status").await.unwrap_or_else(|_| "Normal".to_string());

    let pm1 = PressureMeter {
        id: "pressure-meter-1".to_string(),
        pressure: pm1_pressure,
        status: parse_meter_status(&pm1_status_str),
    };
    devices.insert("pressure-meter-1".to_string(), DeviceState::PressureMeter(pm1));

    // Read Valve-1 data
    let valve1_position = read_node_value(&session, "ns=2;s=PLC1.Valve1.Position").await.unwrap_or(0.0);
    let valve1_mode_str = read_node_string(&session, "ns=2;s=PLC1.Valve1.Mode").await.unwrap_or_else(|_| "Auto".to_string());
    let valve1_status_str = read_node_string(&session, "ns=2;s=PLC1.Valve1.Status").await.unwrap_or_else(|_| "Partial".to_string());

    let valve1 = Valve {
        id: "valve-1".to_string(),
        position: valve1_position,
        mode: parse_valve_mode(&valve1_mode_str),
        status: parse_valve_status(&valve1_status_str),
    };
    devices.insert("valve-1".to_string(), DeviceState::Valve(valve1));

    Ok(devices)
}

async fn read_plc2_data(session: &Arc<RwLock<Session>>) -> Result<HashMap<String, DeviceState>, Box<dyn std::error::Error>> {
    let session = session.read();
    let mut devices = HashMap::new();

    // Read Boiler-2 data
    let boiler2_temp = read_node_value(&session, "ns=2;s=PLC2.Boiler2.Temperature").await.unwrap_or(0.0);
    let boiler2_target_temp = read_node_value(&session, "ns=2;s=PLC2.Boiler2.TargetTemperature").await.unwrap_or(0.0);
    let boiler2_pressure = read_node_value(&session, "ns=2;s=PLC2.Boiler2.Pressure").await.unwrap_or(0.0);
    let boiler2_status_str = read_node_string(&session, "ns=2;s=PLC2.Boiler2.Status").await.unwrap_or_else(|_| "Off".to_string());

    let boiler2 = Boiler {
        id: "boiler-2".to_string(),
        temperature: boiler2_temp,
        target_temperature: boiler2_target_temp,
        pressure: boiler2_pressure,
        status: parse_boiler_status(&boiler2_status_str),
    };
    devices.insert("boiler-2".to_string(), DeviceState::Boiler(boiler2));

    // Read Flow Meter-1 data
    let fm1_flow_rate = read_node_value(&session, "ns=2;s=PLC2.FlowMeter1.FlowRate").await.unwrap_or(0.0);
    let fm1_total_volume = read_node_value(&session, "ns=2;s=PLC2.FlowMeter1.TotalVolume").await.unwrap_or(0.0);
    let fm1_status_str = read_node_string(&session, "ns=2;s=PLC2.FlowMeter1.Status").await.unwrap_or_else(|_| "Normal".to_string());

    let fm1 = FlowMeter {
        id: "flow-meter-1".to_string(),
        flow_rate: fm1_flow_rate,
        total_volume: fm1_total_volume,
        status: parse_flow_meter_status(&fm1_status_str),
    };
    devices.insert("flow-meter-1".to_string(), DeviceState::FlowMeter(fm1));

    Ok(devices)
}

async fn read_node_value(session: &Session, node_id: &str) -> Result<f64, Box<dyn std::error::Error>> {
    let node_id = NodeId::from_str(node_id)?;
    let read_result = session.read(&[ReadValueId::from(node_id)], TimestampsToReturn::Neither, 0.0).await?;

    if let Some(data_value) = read_result.first() {
        if let Some(value) = &data_value.value {
            if let Variant::Double(val) = value {
                return Ok(*val);
            }
        }
    }

    Err("Failed to read value".into())
}

async fn read_node_string(session: &Session, node_id: &str) -> Result<String, Box<dyn std::error::Error>> {
    let node_id = NodeId::from_str(node_id)?;
    let read_result = session.read(&[ReadValueId::from(node_id)], TimestampsToReturn::Neither, 0.0).await?;

    if let Some(data_value) = read_result.first() {
        if let Some(value) = &data_value.value {
            if let Variant::String(val) = value {
                return Ok(val.as_ref().clone());
            }
        }
    }

    Err("Failed to read string value".into())
}

fn parse_boiler_status(status_str: &str) -> BoilerStatus {
    match status_str {
        "Off" => BoilerStatus::Off,
        "Heating" => BoilerStatus::Heating,
        "Steady" => BoilerStatus::Steady,
        "Overheat" => BoilerStatus::Overheat,
        _ => BoilerStatus::Off,
    }
}

fn parse_meter_status(status_str: &str) -> MeterStatus {
    match status_str {
        "Normal" => MeterStatus::Normal,
        "Warning" => MeterStatus::Warning,
        "Critical" => MeterStatus::Critical,
        _ => MeterStatus::Normal,
    }
}

fn parse_flow_meter_status(status_str: &str) -> FlowMeterStatus {
    match status_str {
        "Normal" => FlowMeterStatus::Normal,
        "Low" => FlowMeterStatus::Low,
        "High" => FlowMeterStatus::High,
        _ => FlowMeterStatus::Normal,
    }
}

fn parse_valve_mode(mode_str: &str) -> ValveMode {
    match mode_str {
        "Manual" => ValveMode::Manual,
        "Auto" => ValveMode::Auto,
        _ => ValveMode::Auto,
    }
}

fn parse_valve_status(status_str: &str) -> ValveStatus {
    match status_str {
        "Open" => ValveStatus::Open,
        "Closed" => ValveStatus::Closed,
        "Partial" => ValveStatus::Partial,
        "Fault" => ValveStatus::Fault,
        _ => ValveStatus::Partial,
    }
}
