# r-winreg
Windows Registry Library

## Decoded Value Data
The following are how registry values are currently being decoded. The ValueKey's decode_data method controls the decoding.

| Data Type | Decoded Value |
| --- | --- |
| 0x00000000 [REG_NONE] | Hex string |
| 0x00000001 [REG_SZ] | u16 le or u8 decoded String |
| 0x00000002 [REG_EXPAND_SZ] | u16 le or u8 decoded String |
| 0x00000003 [REG_BINARY] | Hex string |
| 0x00000004 [REG_DWORD_LITTLE_ENDIAN] | i32 |
| 0x00000005 [REG_DWORD_BIG_ENDIAN] | i32 |
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

#### rwinreg 0.1.1 (2017-10-04)
- Changed key paths to use '\\' as path separator.

#### rwinreg 0.1.0 (2017-10-04)
- Added support for all Cell types
