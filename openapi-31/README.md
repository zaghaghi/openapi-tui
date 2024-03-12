# Bootstrap
The idea is to use openapi code generator for rust to generate openapi specification library. To do so, I started with an openapi specification with 
```json
{
    "openapi": "3.1.0",
    "info": {
        "version": "3.1.0",
        "title": "openapi"
    },
    "components": {
        "schemas": {}
    }
}
```

## Build OpenAPI In OpenAPI
1. Download json-schema of [OpenAPI v3.1](https://github.com/OAI/OpenAPI-Specification/blob/main/schemas/v3.1/schema.json)
2. Copy everything from `schema.$def` into the above OpenAPI json inside `components.schemas`
3. Insert `openapi` definition into `components.schemas`
4. Replace every `#/$defs/` with `#/componenst/schemas`
5. Remove every `"$ref": "#/$defs/specification-extensions"`. We don't need them now.
6. Remove every `oneOf`. We will add them later in the generated code.

See the result file [here](./openapi-in-openapi.json).

## Generate Code

```bash
npx -y @openapitools/openapi-generator-cli generate -i openapi-in-openapi.json -g rust -o /tmp/openapi && mv /tmp/openapi/src/models .
```

## Implement The Remaining Part
Generated code is not complete, and don't generate these structs. We will create them by hand :-) .
```
path-item-or-reference
parameter-or-reference
request-body-or-reference
response-or-reference
callbacks-or-reference
example-or-reference
link-or-reference
header-or-reference
security-scheme-or-reference
```