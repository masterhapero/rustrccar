extern crate dualshock4;
extern crate hidapi;
extern crate serialport;

mod halserialport;

use dualshock4::Dualshock4Data;
use hidapi::HidDevice;
use std::env;

use std::sync::mpsc;
use std::sync::mpsc::{Receiver, Sender};
use std::thread;
// use rb::{SpscRb,Consumer,Producer};


// List HID devices from HidApi
fn _list_hid_devices(api: &hidapi::HidApi) {
    for device in api.device_list() {
        println!("{:04x}:{:04x}", device.vendor_id(), device.product_id());
    }
}

// Find VESC serial port name using USB VID / PID pair
fn find_serial_port(vid: u16, pid: u16) -> Option<String> {
    match serialport::available_ports() {
        Ok(ports) => {
            for p in ports {
                match p.port_type {
                    serialport::SerialPortType::UsbPort(info) => {
                        println!(
                            "{} -> USB VID:{:04x} PID:{:04x}",
                            p.port_name, info.vid, info.pid
                        );
                        if info.vid == vid && info.pid == pid {
                            return Some(p.port_name);
                        }
                    }
                    _ => {
                        println!("{} Non-USB port", p.port_name);
                    }
                }
            }
            None
        }
        Err(e) => {
            eprintln!("{:?}", e);
            None
        }
    }
}

// Show all data from Dualshock 4 controller
fn _debug_all_loop(controller: &HidDevice) {
    loop {
        let data = dualshock4::read(&controller).expect("Failed to read data");
        println!("{:?}", data);
    }
}

fn _debug_test_loop(controller: &HidDevice) {
    loop {
        let data = dualshock4::read(&controller).expect("Failed to read data");
        println!(
            "left x: {leftx:<4} y: {lefty:<4} right x: {rightx:<4} y: {righty:<4}",
            leftx = data.analog_sticks.left.x,
            lefty = data.analog_sticks.left.y,
            rightx = data.analog_sticks.right.x,
            righty = data.analog_sticks.right.y
        );
    }
}

// Send events to other thread via channel
/*
fn _start_dualshock_reader_thread_rb(
    controller: HidDevice,
) -> (rb::Consumer<Dualshock4Data>, thread::JoinHandle<()>) {
    let rb = SpscRb::new(4);
    let (prod, cons) = (rb.producer, rb.consumer);
    // let (sender, receiver): (Sender<Dualshock4Data>, Receiver<Dualshock4Data>) = mpsc::channel();
    let handle = thread::spawn(move || {
        loop {
            let data = dualshock4::read(&controller);
            if prod.write(&data).is_err()) {
                println!("Dualshock reader err")
                break;
            }
        }
    });
    (cons, handle)
}
*/

// Send events to other thread via channel
fn start_dualshock_reader_thread(
    controller: HidDevice,
) -> (mpsc::Receiver<Dualshock4Data>, thread::JoinHandle<()>) {
    let (sender, receiver): (Sender<Dualshock4Data>, Receiver<Dualshock4Data>) = mpsc::channel();
    let handle = thread::spawn(move || {
        loop {
            let data = dualshock4::read(&controller);
            if sender.send(data.expect("Failed joystick read")).is_err() {
                break;
            }
        }
    });
    (receiver, handle)
}

fn show_joystick_thread(receiver: mpsc::Receiver<Dualshock4Data>, 
                        mut conn: vesc_comm::VescConnection<halserialport::HalSerialPort, halserialport::HalSerialPort>) {
    while let Ok(data) = receiver.recv() {
        let mut rpm: i32 = 0;
        if data.buttons.r2.analog_value.is_none() || data.buttons.r2.analog_value.unwrap_or(0) == 0 {
            rpm = (data.buttons.l2.analog_value.unwrap_or(0) as f32 * 3000.0_f32 / 255.0_f32) as i32;
        } else if !data.buttons.r2.analog_value.is_none() {
            rpm = (data.buttons.r2.analog_value.unwrap_or(0) as f32 * -3000.0_f32 / 255.0_f32) as i32;
        }
        println!(
            "left x: {leftx:<4} y: {lefty:<4} right x: {rightx:<4} y: {righty:<4} r2: {r2:<4?} l2: {l2:<4?} rpm: {rpm:<4?}",
            leftx = data.analog_sticks.left.x,
            lefty = data.analog_sticks.left.y,
            rightx = data.analog_sticks.right.x,
            righty = data.analog_sticks.right.y,
            r2 = data.buttons.r2.analog_value.unwrap_or(0),
            l2 = data.buttons.r2.analog_value.unwrap_or(0),
            rpm = rpm
        );
        conn.set_rpm(rpm).unwrap();
    }
}

/*
fn show_joystick_thread(cons: rb::Consumer<Dualshock4Data>, 
                        mut conn: vesc_comm::VescConnection<halserialport::HalSerialPort, halserialport::HalSerialPort>) {
    while let Ok(data) = receiver.recv() {
        let mut rpm: i32 = 0;
        if data.buttons.r2.analog_value.is_none() || data.buttons.r2.analog_value.unwrap_or(0) == 0 {
            rpm = (data.buttons.l2.analog_value.unwrap_or(0) as f32 * 3000.0_f32 / 255.0_f32) as i32;
        } else {
            rpm = (data.buttons.r2.analog_value.unwrap_or(0) as f32 * -3000.0_f32 / 255.0_f32) as i32;
        }
        println!(
            "left x: {leftx:<4} y: {lefty:<4} right x: {rightx:<4} y: {righty:<4} r2: {r2:<4?} l2: {l2:<4?} rpm: {rpm:<4?}",
            leftx = data.analog_sticks.left.x,
            lefty = data.analog_sticks.left.y,
            rightx = data.analog_sticks.right.x,
            righty = data.analog_sticks.right.y,
            r2 = data.buttons.r2.analog_value.unwrap_or(0),
            l2 = data.buttons.r2.analog_value.unwrap_or(0),
            rpm = rpm
        );
        conn.set_rpm(rpm).unwrap();
    }
}
*/


// Milestone 2:
// Use Circluar buffer:
// https://github.com/klingtnet/rb

fn main() {
    let _args: Vec<String> = env::args().collect();

    /* VESC connection part */

    let vesc_port = find_serial_port(0x483, 0x5740).expect("Failed to find serial port per USB spec");
    println!("Vesc port found: {}", vesc_port);

    let vesc = serialport::new(vesc_port, 115200)
        // HalSerialPort can handle timeout and packet length is known, less non-blocking attempts
        .timeout(std::time::Duration::from_millis(100)) 
        .open()
        .unwrap();
    let (port1, port2) = ( // Wrappers into embedded_hal types
        halserialport::HalSerialPort::new(vesc.try_clone().unwrap()),
        halserialport::HalSerialPort::new(vesc),
    );
    let mut conn = vesc_comm::VescConnection::new(port1, port2);

    dbg!(conn.get_fw_version()).ok();

    // dbg!(conn.get_values()).ok();
    // let fw_version = conn.get_fw_version().expect("Could not get default version");
    // println!("VESC firmware version major {} minor {} hw {:?}", fw_version.major, fw_version.minor, fw_version.hw);

    /* Joystick loop */
    let api = hidapi::HidApi::new().expect("Failed to create HID API instance.");
    // list_hid_devices(&api);
    let controller = dualshock4::get_device(&api).expect("Failed to open device");
    // debug_all_loop(&controller);
    // debug_test_loop(&controller);
    let (joystick, h1) = start_dualshock_reader_thread(controller);
    show_joystick_thread(joystick, conn);
    let _r1 = h1.join().unwrap();
}
