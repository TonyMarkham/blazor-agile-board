```bash
cd /Users/tony/git/blazor-agile-board/target/debug
````

```bash
./pm project list --pretty
```

```json
{
  "projects": [
    {
      "created_at": 1770178269,
      "description": "",
      "id": "cc8dc131-5a26-489c-829f-fa9fa066c850",
      "key": "PONE",
      "title": "P1",
      "updated_at": 1770178269
    }
  ]
}
```

```bash
./pm work-item create \
  --project-id cc8dc131-5a26-489c-829f-fa9fa066c850 \
  --type task \
  --title "t4" \
  --parent-id fff61c39-423f-485d-98ca-416eb36e9e54 \
  --pretty
```