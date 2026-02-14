use opcua::server::prelude::*;
use std::sync::Arc;
use tokio::sync::RwLock;
use crate::simulator::devices::{Boiler, PressureMeter, Valve};

pub async fn start_plc1_server(
    boiler: Arc<RwLock<Boiler>>,
    pressure_meter: Arc<RwLock<PressureMeter>>,
    valve: Arc<RwLock<Valve>>,
) -> Result<(), Box<dyn std::error::Error>> {
    tracing::info!("Starting PLC-1 OPC UA Server on port 4840");

    let mut server = ServerBuilder::new()
        .application_name("PLC-1")
        .application_uri("urn:PLC1")
        .discovery_urls(vec!["/".into()])
        .create_sample_keypair(true)
        .pki_dir("./pki-plc1")
        .discovery_server_url(None)
        .host_and_port("0.0.0.0", 4840)
        .server()
        .unwrap();

    // Create address space with node IDs
    {
        let address_space = server.address_space();
        let mut address_space = address_space.write();

        // Create PLC-1 folder
        let plc1_folder = address_space
            .add_folder("PLC1", "PLC1", &NodeId::objects_folder_id())
            .unwrap();

        // Create Boiler-1 folder and variables
        let boiler_folder = address_space
            .add_folder("Boiler1", "Boiler1", &plc1_folder)
            .unwrap();

        // Boiler variables
        let boiler_vars = vec![
            Variable::new(&NodeId::new(2, "Boiler1.Temperature"), "Temperature", "Temperature", 0.0_f64),
            Variable::new(&NodeId::new(2, "Boiler1.TargetTemperature"), "TargetTemperature", "TargetTemperature", 0.0_f64),
            Variable::new(&NodeId::new(2, "Boiler1.Pressure"), "Pressure", "Pressure", 0.0_f64),
            Variable::new(&NodeId::new(2, "Boiler1.Status"), "Status", "Status", UAString::from("")),
        ];
        address_space.add_variables(boiler_vars, &boiler_folder);

        // Create Pressure Meter folder and variables
        let pm_folder = address_space
            .add_folder("PressureMeter1", "PressureMeter1", &plc1_folder)
            .unwrap();

        let pm_vars = vec![
            Variable::new(&NodeId::new(2, "PressureMeter1.Pressure"), "Pressure", "Pressure", 0.0_f64),
            Variable::new(&NodeId::new(2, "PressureMeter1.Status"), "Status", "Status", UAString::from("")),
        ];
        address_space.add_variables(pm_vars, &pm_folder);

        // Create Valve folder and variables
        let valve_folder = address_space
            .add_folder("Valve1", "Valve1", &plc1_folder)
            .unwrap();

        let valve_vars = vec![
            Variable::new(&NodeId::new(2, "Valve1.Position"), "Position", "Position", 0.0_f64),
            Variable::new(&NodeId::new(2, "Valve1.Mode"), "Mode", "Mode", UAString::from("")),
            Variable::new(&NodeId::new(2, "Valve1.Status"), "Status", "Status", UAString::from("")),
        ];
        address_space.add_variables(valve_vars, &valve_folder);
    }

    // Spawn update task
    let address_space = server.address_space();
    tokio::spawn(async move {
        let mut interval = tokio::time::interval(tokio::time::Duration::from_millis(100));

        loop {
            interval.tick().await;

            let boiler_data = boiler.read().await;
            let pm_data = pressure_meter.read().await;
            let valve_data = valve.read().await;

            let mut address_space = address_space.write();

            // Update boiler values
            let _ = address_space.set_variable_value(
                NodeId::new(2, "Boiler1.Temperature"),
                boiler_data.temperature,
                &DateTime::now(),
                &DateTime::now(),
            );
            let _ = address_space.set_variable_value(
                NodeId::new(2, "Boiler1.TargetTemperature"),
                boiler_data.target_temperature,
                &DateTime::now(),
                &DateTime::now(),
            );
            let _ = address_space.set_variable_value(
                NodeId::new(2, "Boiler1.Pressure"),
                boiler_data.pressure,
                &DateTime::now(),
                &DateTime::now(),
            );
            let status_str = format!("{:?}", boiler_data.status);
            let _ = address_space.set_variable_value(
                NodeId::new(2, "Boiler1.Status"),
                UAString::from(status_str),
                &DateTime::now(),
                &DateTime::now(),
            );

            // Update pressure meter values
            let _ = address_space.set_variable_value(
                NodeId::new(2, "PressureMeter1.Pressure"),
                pm_data.pressure,
                &DateTime::now(),
                &DateTime::now(),
            );
            let pm_status_str = format!("{:?}", pm_data.status);
            let _ = address_space.set_variable_value(
                NodeId::new(2, "PressureMeter1.Status"),
                UAString::from(pm_status_str),
                &DateTime::now(),
                &DateTime::now(),
            );

            // Update valve values
            let _ = address_space.set_variable_value(
                NodeId::new(2, "Valve1.Position"),
                valve_data.position,
                &DateTime::now(),
                &DateTime::now(),
            );
            let valve_mode_str = format!("{:?}", valve_data.mode);
            let _ = address_space.set_variable_value(
                NodeId::new(2, "Valve1.Mode"),
                UAString::from(valve_mode_str),
                &DateTime::now(),
                &DateTime::now(),
            );
            let valve_status_str = format!("{:?}", valve_data.status);
            let _ = address_space.set_variable_value(
                NodeId::new(2, "Valve1.Status"),
                UAString::from(valve_status_str),
                &DateTime::now(),
                &DateTime::now(),
            );
        }
    });

    server.run();
    Ok(())
}
