extern crate foundationdb;

use std;
use grpc;
use futures::{ Future, Stream };
use ::protogen::demosave_grpc::*;
use ::protogen::demosave::*;

pub struct SaverImpl {
    db: foundationdb::Database,
    thread: Option<std::thread::JoinHandle<()>>,
    fdb_net: foundationdb::network::Network,
    subspace: foundationdb::Subspace,
}

fn now() -> u64{
    let system_time = std::time::SystemTime::now();
    let duration =
        system_time.duration_since(std::time::UNIX_EPOCH).expect("System clock disjunct");

    (duration.as_secs()*1000) + duration.subsec_millis() as u64
}

impl SaverImpl {
    pub fn new () -> SaverImpl {

        let fdb_net = foundationdb::init().expect("Failed to initialize FDB client");
        let fdb_thread = std::thread::spawn(move || {
            let error = fdb_net.run();
            error.unwrap();
        });

        let db = foundationdb::Cluster::new(foundationdb::default_config_path())
            .and_then(|cluster| cluster.create_database())
            .wait().expect("Failed to connect to FDB");

        let subspace = foundationdb::Subspace::from_bytes(b"test_array");

        SaverImpl { thread: Some(fdb_thread), fdb_net, db, subspace }

    }
}

impl Drop for SaverImpl {
    fn drop (&mut self) {
        self.fdb_net.stop().expect("FDB network unable to close gracefully");
        let thread = self.thread.take().expect("FDB thread missing");
        thread.join().expect("FDB thread unable to close gracefully");
    }
}

impl Saver for SaverImpl {
    fn save_value(&self, _m: grpc::RequestOptions, req: SaveRequest) -> grpc::SingleResponse<SaveReply> {

        let value = req.get_value();
        println!("Saving: \"{}\"", value);

        let key = self.subspace.pack(now().to_string());
        let trx = self.db.create_trx().expect("Unable to create save transaction");

        trx.set(&key, value.as_bytes());
        let result = match trx.commit().wait() {
            Ok(_) => SaveReply_Result::OK,
            Err(_) => SaveReply_Result::ERROR,
        };

        let mut r = SaveReply::new();
        r.set_result(result);
        grpc::SingleResponse::completed(r)
    }

    fn list_values(&self, _m: grpc::RequestOptions, _req: ListRequest) -> grpc::SingleResponse<ValueList> {
        let (start, end) = self.subspace.range();
        let start = foundationdb::keyselector::KeySelector::first_greater_or_equal(&start);
        let end = foundationdb::keyselector::KeySelector::last_less_than(&end);
        let range = foundationdb::transaction::RangeOptionBuilder::new(start, end).build();

        let trx = self.db.create_trx().expect("Unable to create list transaction");

        // This is one of the clumsiest interfaces I've ever dealt with.
        let db_stream = trx.get_ranges(range)
            .collect()
            .wait()
            .expect("Could not collect list of values");
        let db_result: Vec<String> = db_stream.into_iter()
            .fold(Vec::new(), |mut acc, range_res| {
                range_res.key_values()
                    .into_iter()
                    .map(|kv| String::from_utf8(kv.value().into()).expect("Malformed strings in DB"))
                    .for_each(|val| acc.push(val));
                acc
            });

        let values = ::protobuf::RepeatedField::from_vec(db_result);
        let mut r = ValueList::new();
        r.set_values(values);
        grpc::SingleResponse::completed(r)
    }
}
