# Examples

This repository houses an example Alvarium Publisher and Receiver application split into 2 packages.
They can be run from the same machine or not, but they are 2 independent packages that encompass 
the lifecycle of a data source annotating and being scored on the other end. 

## Publishing
The publisher package provides an example of an application that produces data from 2 mock sensors. 
This data is generated randomly and produces an sdk instance that contains a set of core annotators 
from the Alvarium rust [SDK](https://github.com/project-alvarium/alvarium-sdk-rust). Additionally, it creates 
a custom Annotator that checks if the values generated are within a specific threshold range. This 
Annotator implements the [Annotator](https://github.com/project-alvarium/alvarium-annotator/blob/main/src/annotator.rs#L3)
trait, and is compatible with the sdk as a result. 

Data is transported through the Demia Distributed Oracle Network (DON for short) which uses the IOTA 
distributed ledger protocol at its foundation, providing auditability and immutability. 

To run this example, simply navigate to the alvarium_demo_pub directory and run 
``` 
cargo run --release 
```

The `--release` flag will ensure that the PoW for the publishing is conducted more efficiently than in dev mode. 

_**Note:**_ _*Configurations are set so that the "provider" is the local application. A demia based streams
implementation is mocked, but a proper oracle would be used in production and the configs would be updated 
accordingly to reflect the oracle address*_

## Subscribing
The subscriber package provides an example of a scoring application that retrieves messages from the publisher
channel, and proceeds to locally store and sort Readings and Annotations. It also spins up a localised web 
application at port 8000 where a visualiser is provided that lets you navigate through the reading history and 
see the evaluated scores and associated annotations for the data sources provided in the publishing process.

To run this example, simply navigate to the alvarium_demo_sub directory and run
``` 
cargo run --release 
```