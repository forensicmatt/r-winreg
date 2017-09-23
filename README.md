# r-winreg
Windows Registry Library

## Decoded Value Data
The following are how registry values are currently being decoded. The ValueKey's decode_data method controls the decoding.

| Data Type | Decoded Value |
| --- | --- |
| 0x00000000 [REG_NONE] | Hex string |
| 0x00000001 [REG_SZ] | u16 le or u8 decoded String |
| All Others | Hex string |

## JSON Key Output
```json
{
  "fullpath": "CsiTool-CreateHive-{00000000-0000-0000-0000-000000000000}/AppEvents/EventLabels/.Default/DispFileName",
  "security": {
    "owner_sid": "S-1-5-18",
    "group_sid": "S-1-5-18",
    "dacl": {
      "revision": 2,
      "count": 5,
      "entries": [
        {
          "ace_type": "ACCESS_ALLOWED",
          "ace_flags": "OBJECT_INHERIT_ACE | CONTAINER_INHERIT_ACE",
          "data": {
            "access_rights": 983103,
            "sid": "S-1-5-21-718126207-1171771683-1750804747-1001"
          }
        },
        {
          "ace_type": "ACCESS_ALLOWED",
          "ace_flags": "OBJECT_INHERIT_ACE | CONTAINER_INHERIT_ACE",
          "data": {
            "access_rights": 983103,
            "sid": "S-1-5-18"
          }
        },
        {
          "ace_type": "ACCESS_ALLOWED",
          "ace_flags": "OBJECT_INHERIT_ACE | CONTAINER_INHERIT_ACE",
          "data": {
            "access_rights": 983103,
            "sid": "S-1-5-32-544"
          }
        },
        {
          "ace_type": "ACCESS_ALLOWED",
          "ace_flags": "OBJECT_INHERIT_ACE | CONTAINER_INHERIT_ACE",
          "data": {
            "access_rights": 131097,
            "sid": "S-1-5-12"
          }
        },
        {
          "ace_type": "ACCESS_ALLOWED",
          "ace_flags": "OBJECT_INHERIT_ACE | CONTAINER_INHERIT_ACE",
          "data": {
            "access_rights": 131097,
            "sid": "S-1-15-2-1"
          }
        }
      ]
    }
  },
  "value": {
    "data_size": 34,
    "data_type": "REG_SZ",
    "flags": "VK_VALUE_COMP_NAME",
    "value_name": "DispFileName",
    "data": "@mmres.dll,-5824"
  }
}
```
