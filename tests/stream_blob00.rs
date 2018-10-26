#![cfg(all(test, feature = "test_e2e"))]

extern crate azure_sdk_for_rust;
extern crate chrono;
extern crate futures;
extern crate hyper;
extern crate tokio_core;

use azure_sdk_for_rust::core::{range::Range, DeleteSnapshotsMethod};
use azure_sdk_for_rust::prelude::*;
use azure_sdk_for_rust::storage::client::Client;
use azure_sdk_for_rust::storage::client::Container as ContainerTrait;
use azure_sdk_for_rust::storage::container::{PublicAccess, PublicAccessSupport};
use futures::future::ok;
use futures::prelude::*;
use tokio_core::reactor::Core;

#[test]
fn create_blob_and_stream_back() {
    code().unwrap();
}

fn code() -> Result<(), Box<std::error::Error>> {
    let container_name = "azuresdkforrust";
    let file_name = "azure_sdk_for_rust_stream_test.txt";

    // First we retrieve the account name and master key from environment variables.
    let account = std::env::var("STORAGE_ACCOUNT").expect("Set env variable STORAGE_ACCOUNT first!");
    let master_key = std::env::var("STORAGE_MASTER_KEY").expect("Set env variable STORAGE_MASTER_KEY first!");

    let mut reactor = Core::new()?;
    let client = Client::new(Account::Azure { account, key: master_key })?;

    if reactor
        .run(client.list_containers().finalize())?
        .incomplete_vector
        .iter()
        .find(|x| x.name == container_name)
        .is_none()
    {
        reactor.run(
            client
                .create_container()
                .with_container_name(container_name)
                .with_public_access(PublicAccess::Blob)
                .finalize(),
        )?;
    }

    let string = "0123456789ABCDEF0123456789ABCDEF0123456789ABCDEF0123456789ABCDEF0123456789ABCDEF0123456789ABCDEF0123456789ABCDEF0123456789ABCDEF0123456789ABCDEF0123456789ABCDEF0123456789ABCDEF0123456789ABCDEF0123456789ABCDEF0123456789ABCDEF0123456789ABCDEF0123456789ABCDEF";

    let fut = client
        .put_block_blob()
        .with_container_name(&container_name)
        .with_blob_name(file_name)
        .with_content_type("text/plain")
        .with_body(string.as_ref())
        .finalize();

    let fut = fut.map(|_| {
        println!("{}/{} blob created!", container_name, file_name);
    });
    reactor.run(fut)?;

    // this is how you stream data from azure blob. Notice that you have
    // to specify the range requested. Also make sure to specify how big
    // a chunk is going to be. Bigger chunks are of course more efficient as the
    // http overhead will be less but it also means you will have to wait for more
    // time before receiving anything. In this example we use an awkward value
    // just to make the test worthwile.
    let stream = client
        .stream_blob()
        .with_container_name(&container_name)
        .with_blob_name(file_name)
        .with_range(&Range::new(0, string.len() as u64))
        .finalize();

    let result = std::rc::Rc::new(std::cell::RefCell::new(Vec::new()));

    {
        let mut res_closure = result.borrow_mut();
        let fut = stream.for_each(move |mut value| {
            println!("received {:?} bytes", value.len());
            res_closure.append(&mut value);

            ok(())
        });

        reactor.run(fut)?;
    }

    let returned_string = {
        let mut rlock = result.borrow_mut();
        String::from_utf8(rlock.to_vec())?
    };

    // You can of course conctenate all the
    // pieces as shown below.
    // It generally does not make sense as you
    // will lose the ability to process the data as it
    // comes in.
    //
    //let fut = stream.concat2().map(|res| {
    //    println!("all blocks received");
    //    res
    //});
    //
    //let result = reactor.run(fut)?;
    //let returned_string = String::from_utf8(result)?;

    println!("{}", returned_string);

    assert!(
        string == returned_string,
        "string = {}, returned_string = {}",
        string,
        returned_string
    );

    let future = client
        .delete_blob()
        .with_container_name(&container_name)
        .with_blob_name(file_name)
        .with_delete_snapshots_method(DeleteSnapshotsMethod::Include)
        .finalize()
        .map(|_| {
            println!("{}/{} blob deleted!", container_name, file_name);
        });

    reactor.run(future)?;

    Ok(())
}
