Convert minecraft chunk packet (https://wiki.vg/Chunk_Format) obtained using https://github.com/asyade/cort2bot into a playable map

# Compatibility
Version 1.15.x

## USAGE
```dump-to-map -o <output> [SUBCOMMAND]```

## FLAGS:
* `-h`, `--help`       Prints help information
* `-V`, `--version`    Prints version information

## OPTIONS:
* -o <output>        Minecraft region directory

## SUBCOMMANDS
### `bulk`
Copy a bunch of json chunk sections into an existing minecraft world
```dump-to-map -o <output> bulk --patch <patch>```
#### OPTIONS
* `-p`, `--patch <patch>`    A directory containing JOSN chunk regions

### `find`
Find coords of a block
Copy a bunch of json chunk sections into an existing minecraft world
```dump-to-map -o <output> find [FLAGS] [OPTIONS]```
#### OPTIONS
* `-f`, `--force`      Rescue from crash on corupted file but extremly slow
* `-h`, `--help`       Prints help information
* `-l`, `--list`       List available blocks

### `listen`
Listen for chunk sections over a websocket and apply them to an existing minecraft world
Copy a bunch of json chunk sections into an existing minecraft world
```dump-to-map -o <output> listen [OPTIONS]```
#### OPTIONS
* `-p`, `--port <port>`    Listen port [default: 4242]
