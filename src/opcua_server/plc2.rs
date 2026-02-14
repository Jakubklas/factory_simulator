use opcua::server::prelude::*;
use std::sync::Arc;
use tokio::sync::RwLock;
use crate::simulator::devices::{Boiler, FlowMeter};

pub async fn start_plc2_server(
    boiler: Arc<RwLock<Boiler>>,
    flow_meter: Arc<RwLock<FlowMeter>>,
) -> Result<(), Box<dyn std::error::Error>> {
    tracing::info!("Starting PLC-2 OPC UA Server on port 4841");

    let mut server = ServerBuilder::new()
        .application_name("PLC-2")
        .application_uri("urn:PLC2")
        .discovery_urls(vec!["/".into()])
        .create_sample_keypair(true)
        .pki_dir("./pki-plc2")
        .discovery_server_url(None)
        .host_and_port("0.0.0.0", 4841)
        .server()
        .unwrap();

    // Create address space with node IDs
    {
        let address_space = server.address_space();
        let mut address_space = address_space.write();

        // Create PLC-2 folder
        let plc2_folder = address_space
            .add_folder("PLC2", "PLC2", &NodeId::objects_folder_id())
            .unwrap();

        // Create Boiler-2 folder and variables
        let boiler_folder = address_space
            .add_folder("Boiler2", "Boiler2", &plc2_folder)
            .unwrap();

        // Boiler variables
        let boiler_vars = vec![
            Variable::new(&NodeId::new(2, "Boiler2.Temperature"), "Temperature", "Temperature", 0.0_f64),
            Variable::new(&NodeId::new(2, "Boiler2.TargetTemperature"), "TargetTemperature", "TargetTemperature", 0.0_f64),
            Variable::new(&NodeId::new(2, "Boiler2.Pressure"), "Pressure", "Pressure", 0.0_f64),
            Variable::new(&NodeId::new(2, "Boiler2.Status"), "Status", "Status", UAString::from("")),
        ];
        address_space.add_variables(boiler_vars, &boiler_folder);

        // Create Flow Meter folder and variables
        let fm_folder = address_space
            .add_folder("FlowMeter1", "FlowMeter1", &plc2_folder)
            .unwrap();

        let fm_vars = vec![
            Variable::new(&NodeId::new(2, "FlowMeter1.FlowRate"), "FlowRate", "FlowRate", 0.0_f64),
            Variable::new(&NodeId::new(2, "FlowMeter1.TotalVolume"), "TotalVolume", "TotalVolume", 0.0_f64),
            Variable::new(&NodeId::new(2, "FlowMeter1.Status"), "Status", "Status", UAString::from("")),
        ];
        address_space.add_variables(fm_vars, &fm_folder);
    }

    // Spawn update task
    let address_space = server.address_space();
    tokio::spawn(async move {
        let mut interval = tokio::time::interval(tokio::time::Duration::from_millis(100));

        loop {
            interval.tick().await;

            let boiler_data = boiler.read().await;
            let fm_data = flow_meter.read().await;

            let mut address_space = address_space.write();

            // Update boiler values
            let _ = address_space.set_variable_value(
                NodeId::new(2, "Boiler2.Temperature"),
                boiler_data.temperature,
                &DateTime::now(),
                &DateTime::now(),
            );
            let _ = address_space.set_variable_value(
                NodeId::new(2, "Boiler2.TargetTemperature"),
                boiler_data.target_temperature,
                &DateTime::now(),
                &DateTime::now(),
            );
            let _ = address_space.set_variable_value(
                NodeId::new(2, "Boiler2.Pressure"),
                boiler_data.pressure,
                &DateTime::now(),
                &DateTime::now(),
            );
            let status_str = format!("{:?}", boiler_data.status);
            let _ = address_space.set_variable_value(
                NodeId::new(2, "Boiler2.Status"),
                UAString::from(status_str),
                &DateTime::now(),
                &DateTime::now(),
            );

            // Update flow meter values
            let _ = address_space.set_variable_value(
                NodeId::new(2, "FlowMeter1.FlowRate"),
                fm_data.flow_rate,
                &DateTime::now(),
                &DateTime::now(),
            );
            let _ = address_space.set_variable_value(
                NodeId::new(2, "FlowMeter1.TotalVolume"),
                fm_data.total_volume,
                &DateTime::now(),
                &DateTime::now(),
            );
            let fm_status_str = format!("{:?}", fm_data.status);
            let _ = address_space.set_variable_value(
                NodeId::new(2, "FlowMeter1.Status"),
                UAString::from(fm_status_str),
                &DateTime::now(),
                &DateTime::now(),
            );
        }
    });

    server.run();
    Ok(())
}
