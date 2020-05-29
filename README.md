# rs_json_transform_worker
Transform Json to Json using template (based on jq tool)

## Supports

* jq 1.5 transformations
* conversions between xml and json

## Warning

Usage of xml conversions is not straightforward, the json transformation must respect the [jxon conventions](https://github.com/definitelynobody/jxon).

Examples:

| xml | json |
| :-: | :-: |
| `<?xml version="1.0" encoding="UTF-8"?>` | `{"#": {"version": "1.0", "encoding": "UTF-8"}}` |
| `<root Something="value"/>` | `{"root":[{"$Something": "value"}]}` |
| `<name type="str">John Doe</name>` | `{"name":[{"_": "John Doe","$type": "str"}]}`|