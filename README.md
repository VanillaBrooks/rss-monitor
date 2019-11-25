### Importing tables from scratch

The following command will import `database.sql` and setup tables required by the binary in an already created `database_name`:

```psql -U username -d database_name -a -f database.sql```

where ```database.sql``` is located in ```src/database/database.sql```