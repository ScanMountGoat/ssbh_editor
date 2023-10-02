pub fn device_info_window(ctx: &egui::Context, open: &mut bool, info: &wgpu::AdapterInfo) {
    egui::Window::new("Device Info")
        .resizable(false)
        .open(open)
        .show(ctx, |ui| {
            egui::Grid::new("device_info").show(ui, |ui| {
                ui.label("Name");
                ui.label(&info.name);
                ui.end_row();

                ui.label("Vendor");
                ui.label(info.vendor.to_string());
                ui.end_row();

                ui.label("Device");
                ui.label(info.device.to_string());
                ui.end_row();

                ui.label("Device Type");
                ui.label(format!("{:?}", info.device_type));
                ui.end_row();

                ui.label("Driver");
                ui.label(&info.driver);
                ui.end_row();

                ui.label("Driver Info");
                ui.label(&info.driver_info);
                ui.end_row();

                ui.label("Backend");
                ui.label(format!("{:?}", info.backend));
                ui.end_row();
            });
        });
}
