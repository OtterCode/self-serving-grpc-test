extern crate protobuf;
extern crate grpc;
extern crate futures;
extern crate futures_cpupool;
extern crate tls_api;
extern crate tls_api_native_tls;
extern crate httpbis;

mod protogen;
mod saver;

use protogen::demosave::*;
use saver::SaverImpl;
use protogen::demosave_grpc::*;
use tls_api::{ TlsAcceptorBuilder, TlsConnector, TlsConnectorBuilder }; // TLS
use std::fs::File; // TLS
use std::io::{ Read, stdin }; // TLS
use std::sync::Arc;
use std::net::SocketAddr;


fn main() {
    let port = 50052;

    let saver = SaverImpl::new();

    let cert: Result<Vec<u8>, _> = File::open("./certificate.p12")
        .expect("Cert file not found")
        .bytes()
        .collect();
    let cert: Vec<u8> = cert.expect("Cert file corrupted");


    let builder = tls_api_native_tls::TlsAcceptorBuilder::from_pkcs12(&cert, "")
        .expect("TLS builder corrupt");
    let tls_acceptor = builder.build().expect("TLS configuration corrupt");

    let mut server = grpc::ServerBuilder::<tls_api_native_tls::TlsAcceptor>::new();
    server.http.set_port(port);
    server.add_service(SaverServer::new_service_def(saver));
    server.http.set_cpu_pool_threads(4);
    server.http.set_tls(tls_acceptor); // TLS

    let _server = server.build().expect("server");


    // gRPC Client ===============================================================================

    let client_cert: Result<Vec<u8>, _> = File::open("./certificate.der")
        .expect("Client cert file not found")
        .bytes()
        .collect();
    let client_cert =
        tls_api::Certificate::from_der(client_cert.expect("Client cert file corrupted"));


    let mut client_builder =
        tls_api_native_tls::TlsConnector::builder().expect("Client TLS builder corrupted");
    client_builder.add_root_certificate(client_cert).expect("Cannot add root cert to client");
    let client_tls = client_builder.build().expect("Client TLS builder corrupt");

    let client_conf = Default::default();

    let tls_option = httpbis::ClientTlsOption::Tls(
    "localhost".to_owned(), Arc::new(client_tls));
    let addr = SocketAddr::new("::1".parse().unwrap(), port);
    let grpc_client = grpc::Client::new_expl(&addr, "localhost", tls_option, client_conf).unwrap();
    let client = SaverClient::with_client(grpc_client);

    //
    static EXIT: &str = ":exit";
    loop {

        let mut input = String::new();
        println!("Input value to send and save, then hit enter, or type \"{}\" to quit: ", EXIT);

        stdin().read_line(&mut input).expect("Error on user input");
        input = input.trim_right().into();

        if input == EXIT { break; }

        let mut save_req = SaveRequest::new();
        save_req.set_value(input);

        let list_resp = client.list_values(grpc::RequestOptions::new(), ListRequest::new()).wait();
        println!("Previous Values: {:?}", list_resp);

        let save_resp = client.save_value(grpc::RequestOptions::new(), save_req).wait();
        println!("Save Response: {:?}", save_resp);

    }
}
