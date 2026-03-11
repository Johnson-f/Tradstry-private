## Server Management Commands

### SSH Connection
```bash
ssh -i ~/.ssh/id_ed25519_vps root@95.216.219.131
```

### View Backend Logs
```bash
docker logs tradstry-backend
```

### Restart Backend Service
```bash
systemctl restart tradstry-backend
```

### Update Deployment
Run this command after making code changes:
```bash
cd /opt/tradstry && docker compose -f docker-compose.backend.yaml up -d --build
```
