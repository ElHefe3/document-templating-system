# Document Templating System Bruno Tests

Run the app first:

```powershell
scripts\start-document-templating-system.cmd --web --no-open
```

Then run the collection:

```powershell
cd bruno\document-templating-system-web
bru run --env-file environments\local.bru
```

The shared repo runner starts a temporary workspace automatically:

```powershell
py scripts\test_all.py
```

Bruno still supports `.bru` collections. The request files are ordered so CLI and GUI runs exercise the same core flow.
