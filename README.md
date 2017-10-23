# r-winreg
Windows Registry Library (Does not use Windows API)

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

## Record Output
```json
{
  "fullpath": "\\CsiTool-CreateHive-{00000000-0000-0000-0000-000000000000}\\7-Zip\\Path",
  "nk_last_written": "2013-10-18 00:28:34.576",
  "valuekey": {
    "data_size": 48,
    "data_type": "REG_SZ",
    "flags": "VK_VALUE_COMP_NAME",
    "value_name": "Path",
    "data": "C:\\Program Files\\7-Zip\\"
  },
  "security": {
    "owner_sid": "S-1-5-18",
    "group_sid": "S-1-5-18",
    "dacl": {
      "revision": 2,
      "count": 10,
      "entries": [
        {
          "ace_type": "ACCESS_ALLOWED",
          "ace_flags": "(empty)",
          "data": {
            "access_rights": 131097,
            "sid": "S-1-5-32-545"
          }
        },
        {
          "ace_type": "ACCESS_ALLOWED",
          "ace_flags": "CONTAINER_INHERIT_ACE | INHERIT_ONLY_ACE",
          "data": {
            "access_rights": 2147483648,
            "sid": "S-1-5-32-545"
          }
        },
        {
          "ace_type": "ACCESS_ALLOWED",
          "ace_flags": "(empty)",
          "data": {
            "access_rights": 983103,
            "sid": "S-1-5-32-544"
          }
        },
        {
          "ace_type": "ACCESS_ALLOWED",
          "ace_flags": "CONTAINER_INHERIT_ACE | INHERIT_ONLY_ACE",
          "data": {
            "access_rights": 268435456,
            "sid": "S-1-5-32-544"
          }
        },
        {
          "ace_type": "ACCESS_ALLOWED",
          "ace_flags": "(empty)",
          "data": {
            "access_rights": 983103,
            "sid": "S-1-5-18"
          }
        },
        {
          "ace_type": "ACCESS_ALLOWED",
          "ace_flags": "CONTAINER_INHERIT_ACE | INHERIT_ONLY_ACE",
          "data": {
            "access_rights": 268435456,
            "sid": "S-1-5-18"
          }
        },
        {
          "ace_type": "ACCESS_ALLOWED",
          "ace_flags": "(empty)",
          "data": {
            "access_rights": 983103,
            "sid": "S-1-5-18"
          }
        },
        {
          "ace_type": "ACCESS_ALLOWED",
          "ace_flags": "CONTAINER_INHERIT_ACE | INHERIT_ONLY_ACE",
          "data": {
            "access_rights": 268435456,
            "sid": "S-1-3-0"
          }
        },
        {
          "ace_type": "ACCESS_ALLOWED",
          "ace_flags": "(empty)",
          "data": {
            "access_rights": 131097,
            "sid": "S-1-15-2-1"
          }
        },
        {
          "ace_type": "ACCESS_ALLOWED",
          "ace_flags": "CONTAINER_INHERIT_ACE | INHERIT_ONLY_ACE",
          "data": {
            "access_rights": 2147483648,
            "sid": "S-1-15-2-1"
          }
        }
      ]
    }
  }
}
```

#### rwinreg 0.2.0 (2017-10-22)
- Major re-write using proper reference passing and struct parsing via buffer slices.

#### rwinreg 0.1.2 (2017-10-04)
- Changed key paths to use '\\' as path separator.

#### rwinreg 0.1.0 (2017-10-04)
- Added support for all Cell types
