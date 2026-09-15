
# Compound File

Reader d'un [Compound File][CFBF] selon [MS-CBF][MS-CFB].

[CFBF]: https://en.wikipedia.org/wiki/Compound_File_Binary_Format
[MS-CFB]: https://learn.microsoft.com/en-us/openspecs/windows_protocols/ms-cfb/53989ce4-7b05-4f8d-829b-d08d6148375b

## TODO

### Un jour

- Quid des chaînes cycliques malicieuses ?
- Vérifier tous les calculs avec du `T::checked_XXX()`.
- Prendre en compte l'endianness de la machine hôte.

## Box Drawing

```txt
┌─┬┐  ╔═╦╗  ╓─╥╖  ╒═╤╕
│ ││  ║ ║║  ║ ║║  │ ││
├─┼┤  ╠═╬╣  ╟─╫╢  ╞═╪╡
└─┴┘  ╚═╩╝  ╙─╨╜  ╘═╧╛
```
