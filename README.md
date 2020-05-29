# rs_json_transform_worker
Transform Json to Json using template (based on jq tool)

## Supports

* jq 1.5 transformations
* conversions between xml and json

## Examples

All orders in the `examples` directory can be run without any other service.
In a terminal just to:
```bash
RUST_LOG=debug SOURCE_ORDERS=examples/xml_to_xml.json cargo run
```

More information can be found [here](https://docs.rs/mcai_worker_sdk/0.10.3/mcai_worker_sdk/#start-worker-locally)

## Warning

Usage of xml conversions is not straightforward, the json transformation must respect the [jxon conventions](https://github.com/definitelynobody/jxon).

Examples:

| xml | json |
| :-: | :-: |
| `<?xml version="1.0" encoding="UTF-8"?>` | `{"#": {"version": "1.0", "encoding": "UTF-8"}}` |
| `<root Something="value"/>` | `{"root":[{"$Something": "value"}]}` |
| `<name type="str">John Doe</name>` | `{"name":[{"_": "John Doe","$type": "str"}]}`|