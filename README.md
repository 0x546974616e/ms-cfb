
# Compound File

[Compound File][CFBF] reader-only according to [MS-CFB][MS-CFB].

[CFBF]: https://en.wikipedia.org/wiki/Compound_File_Binary_Format
[MS-CFB]: https://learn.microsoft.com/en-us/openspecs/windows_protocols/ms-cfb/
[MS-OLEPS]: https://learn.microsoft.com/en-us/openspecs/windows_protocols/ms-oleps/

## To use

### List

```sh
mscfb list ./path/to/VSCodium-x64-updates-disabled-1.126.04524.msi
```

```txt
Rr--r--r--  45.7K 2026-Jul  7 15:23 .
-r--r--r--      4 0000-??? ?? 00:00 䡀䆒䑲
-r--r--r--  94.3K 0000-??? ?? 00:00 䡀䌏䈯
dr--r--r--      0 2026-Jul  7 14:15 1028
dr--r--r--      0 2026-Jul  7 14:12 1031
dr--r--r--      0 2026-Jul  7 14:13 1036
[...]
-r--r--r-- 461.9K 0000-??? ?? 00:00 䌋䄱䜵䀾䛬㲞㫿䓰㭿䄬䒯䠪
-r--r--r--    766 0000-??? ?? 00:00 䌋䄱䜵䀾䛬㲞㲿䒦㮿䆻䄯䠰
-r--r--r--    216 0000-??? ?? 00:00 䡀䑒䗶䏤㮯䈻䘦䈷䈜䘴䑨䈦
-r--r--r--  12.1K 0000-??? ?? 00:00 \x{05}DigitalSignature
-r--r--r--    596 0000-??? ?? 00:00 \x{05}SummaryInformation
-r--r--r--     32 0000-??? ?? 00:00 \x{05}MsiDigitalSignatureEx
```

```sh
mscfb list ./path/to/VSCodium-x64-updates-disabled-1.126.04524.msi /1031
```

```txt
dr--r--r--      0 2026-Jul  7 14:12 .
-r--r--r--    276 0000-??? ?? 00:00 䡀㲞䈝䗻
-r--r--r--    144 0000-??? ?? 00:00 䡀䌍䏤䊲
-r--r--r--     48 0000-??? ?? 00:00 䡀䈏䗤䕸䠨
-r--r--r--   1318 0000-??? ?? 00:00 䡀䒌䗱䒵䠯
-r--r--r--     12 0000-??? ?? 00:00 䡀䕙䓲䕨䜷
-r--r--r--  13.8K 0000-??? ?? 00:00 䡀㼿䕷䑬㭪䗤䠤
-r--r--r--   1296 0000-??? ?? 00:00 䡀㼿䕷䑬㹪䒲䠯
-r--r--r--     32 0000-??? ?? 00:00 䡀䄛䌧㫲䗸䒷䠱
-r--r--r--      6 0000-??? ?? 00:00 䡀䘌䗶䐲䆊䌷䑲
-r--r--r--     12 0000-??? ?? 00:00 䡀䄕䑸䋦䒌䇱䗬䒬䠱
-r--r--r--    660 0000-??? ?? 00:00 \x{05}SummaryInformation
```

A few notes on the result:

- The characters `R`, `d`, and `-` indicate, respectively, the *Root Entry*, a
  directory (a *storage* in the Compound File), and a file (a *stream* in the
  Compound File).

- These characters `䡀䑒䗶䏤㮯䈻䘦䈷䈜䘴䑨䈦` (founded in `.msi` files) form a
  valid UTF-16 string and encode a binary payload (otherwise, encoding the data
  as base64 in UTF-16 would double its size).

- Files beginning with the byte `0x05` represent [property sets][MS-OLEPS].

- The specification requires the date to be all zeroes for streams.

### Tree


```sh
mscfb tree ./path/to/VSCodium-x64-updates-disabled-1.126.04524.msi /
```

```txt
+ Root Entry (46.3K)
  ~ 䡀䆒䑲 (144)
  ~ 䡀䌏䈯 (46.6K)
  ~ 1028 (0)
    ~ 䡀㲞䈝䗻 (707)
    ~ 䡀䌍䏤䊲 (704)
    ~ 䡀䈏䗤䕸䠨 (703)
    ~ 䡀䒌䗱䒵䠯 (681)
    ~ 䡀䕙䓲䕨䜷 (680)
    ~ 䡀䈝䗻䗜䏼䠨 (679)
    ~ 䡀㼿䕷䑬㭪䗤䠤 (46.9K)
    ~ 䡀㼿䕷䑬㹪䒲䠯 (659)
    ~ 䡀䄛䌧㫲䗸䒷䠱 (658)
    ~ \x{05}SummaryInformation (647)
  ~ 1031 (0)
    ~ 䡀㲞䈝䗻 (256)
    ~ 䡀䌍䏤䊲 (253)
    ~ 䡀䈏䗤䕸䠨 (252)
[...]
  ~ 䌋䄱䜵䀾䛬㲞㫿䓰㭿䄬䒯䠪 (46.4K)
  ~ 䌋䄱䜵䀾䛬㲞㲿䒦㮿䆻䄯䠰 (13)
  ~ 䡀䑒䗶䏤㮯䈻䘦䈷䈜䘴䑨䈦 (145)
  ~ \x{05}DigitalSignature (46.9K)
  ~ \x{05}SummaryInformation (186)
  ~ \x{05}MsiDigitalSignatureEx (713)
```

### Cat

```sh
mscfb cat ./path/to/VSCodium-x64-updates-disabled-1.126.04524.msi /1031/䡀㼿䕷䑬㭪䗤䠤
```

```txt
WIXUI_EXITDIALOGOPTIONALCHECKBOXTEXTVSCodium
ausf�hrenProductLanguage1031FatalErrorDescriptionDer Setup-Assistent f�r
[ProductName] wurde aufgrund eines Fehlers vorzeitig beendet. Das System wurde
nicht ver�ndert. Sie m�ssen den Setup-Assistenten erneut ausf�hren, um dieses
Programm zu einem sp�teren Zeitpunkt zu installieren. Klicken Sie auf "Fertig
stellen", um den Setup-Assistenten zu beenden.Title{\WixUI_Font_Bigger}Der
Setup-Assistent f�r [ProductName] wurde vorzeitig
beendet.CancelAbbrechenFinish&Fertig stellenBack&Zur�ckUserExitDie
[...]
```

### Unpack

```sh
mscfb unpack ./path/to/VSCodium-x64-updates-disabled-1.126.04524.msi -o /tmp/example
```

```txt
/tmp/example/䡀䆒䑲
/tmp/example/䡀䌏䈯
/tmp/example/1028/䡀㲞䈝䗻
/tmp/example/1028/䡀䌍䏤䊲
/tmp/example/1028/䡀䈏䗤䕸䠨
/tmp/example/1028/䡀䒌䗱䒵䠯
/tmp/example/1028/䡀䕙䓲䕨䜷
/tmp/example/1028/䡀䈝䗻䗜䏼䠨
[...]
```

## Nightly rustfmt

```sh
rustup toolchain install nightly
rustup component add rustfmt --toolchain nightly
```

(Or `rustup toolchain install nightly --component rustfmt`.)

```sh
cargo +nightly fmt
```

The project uses `nightly` for code formatting.
