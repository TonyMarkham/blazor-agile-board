# Troubleshooting

## Build Issues

### "Package not found" or missing dependencies
Run:
```bash
just restore
```

### Frontend build fails after pulling changes
Run:
```bash
just restore-frontend
just build-frontend
```

### Backend build fails after pulling changes
Run:
```bash
just restore-backend
just build-backend
```

---

## Config Issues

### "config.toml not found"
Run:
```bash
just setup-config
```

This creates `.server/config.toml` from `backend/config.example.toml`.

---

## WebSocket Issues

### "Disconnected" status
- Ensure the backend is running
- Check `backend/config.example.toml` for port settings
- Verify no firewall or proxy is blocking the WebSocket

### Updates not appearing
- Confirm the client has subscribed to the project
- Check the WebSocket connection state indicator

---

## Tests Failing

Run the specific test suite:

```bash
just test-cs-core
just test-cs-services
just test-cs-components
```

---

## Still Stuck?

- Run `just help` to see full command list
- Review `docs/ARCHITECTURE.md` and `docs/websocket-protocol.md`
