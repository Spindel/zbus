#![deny(rust_2018_idioms)]

use std::{
    env,
    error::Error,
    fs::File,
    io::Write,
    path::Path,
    process::{Command, Stdio},
    result::Result,
};

use zbus::xml::{Interface, Node};

mod gen;
use gen::GenTrait;

fn main() -> Result<(), Box<dyn Error>> {
    let args: Vec<String> = env::args().collect();
    if args.len() != 2 {
        println!("usage: zbus-xmlgen <interface.xml>");
        return Ok(());
    }

    let inputpath = &args[1];
    let f = File::open(inputpath)?;

    let node = Node::from_reader(f)?;
    let mut process = match Command::new("rustfmt").stdin(Stdio::piped()).spawn() {
        Err(why) => panic!("couldn't spawn rustfmt: {}", why),
        Ok(process) => process,
    };
    let rustfmt_stdin = process.stdin.as_mut().unwrap();
    let fdo_iface_prefix = "org.freedesktop.DBus";
    let (fdo_standard_ifaces, needed_ifaces): (Vec<&Interface>, Vec<&Interface>) = node
        .interfaces()
        .iter()
        .partition(|&&i| i.name().starts_with(fdo_iface_prefix));

    if let Some((first_iface, following_ifaces)) = needed_ifaces.split_first() {
        if following_ifaces.is_empty() {
            writeln!(
                rustfmt_stdin,
                "//! # DBus interface proxy for: `{}`",
                first_iface.name()
            )?;
        } else {
            write!(
                rustfmt_stdin,
                "//! # DBus interface proxies for: `{}`",
                first_iface.name()
            )?;
            for iface in following_ifaces {
                write!(rustfmt_stdin, ", `{}`", iface.name())?;
            }
            writeln!(rustfmt_stdin)?;
        }
    }

    write!(
        rustfmt_stdin,
        "//!
         //! This code was generated by `{}` `{}` from DBus introspection data
         //! in: `{}`.
         //!
         //! You may prefer to adapt it, instead of using it verbatim.
         //!
         //! More information can be found in the
         //! [Writing a client proxy](https://zeenix.pages.freedesktop.org/zbus/client.html)
         //! section of the zbus documentation.
         //!
        ",
        env!("CARGO_BIN_NAME"),
        env!("CARGO_PKG_VERSION"),
        Path::new(inputpath).file_name().unwrap().to_string_lossy(),
    )?;
    if !fdo_standard_ifaces.is_empty() {
        write!(rustfmt_stdin,
            "//! This DBus object implements
             //! [standard DBus interfaces](https://dbus.freedesktop.org/doc/dbus-specification.html),
             //! (`org.freedesktop.DBus.*`) for which the following zbus proxies can be used:
             //!
            ")?;
        for iface in &fdo_standard_ifaces {
            let idx = iface.name().rfind('.').unwrap() + 1;
            let name = &iface.name()[idx..];
            writeln!(rustfmt_stdin, "//! * [`zbus::fdo::{}Proxy`]", name)?;
        }
        write!(
            rustfmt_stdin,
            "//!
             //! …consequently `{}` did not generate code for the above interfaces.
            ",
            env!("CARGO_BIN_NAME")
        )?;
    }
    write!(
        rustfmt_stdin,
        "
        use zbus::dbus_proxy;
        "
    )?;
    for iface in &needed_ifaces {
        writeln!(rustfmt_stdin)?;
        let gen = GenTrait(&iface).to_string();
        rustfmt_stdin.write_all(gen.as_bytes())?;
    }
    process.wait()?;
    Ok(())
}
