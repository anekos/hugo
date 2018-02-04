# Hugo

flag manager for shell script

```
$ hugo

Usage: hugo <DB_NAME> has <ID>
       hugo <DB_NAME> get <ID> [<DEFAULT>]
       hugo <DB_NAME> set <ID> [<TEXT>]
       hugo <DB_NAME> check <ID> [<TEXT>]
       hugo <DB_NAME> swap <ID> [<TEXT>]

has:
  If <DATA_FILE> has <ID>, `hugo` command succeeds.
get:
  If <DATA_FILE> has <ID>, `hugo` command succeeds and print the value.
set:
  Set <TEXT> with <ID>.
check:
  `has` and `set`
swap:
  Same to `check` except that print the old value.
```
