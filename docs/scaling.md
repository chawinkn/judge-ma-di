# Scaling & Cloud Deployment Guide

This guide explains how to scale Judge Ma Di horizontally across multiple workers, run distinct service roles, and deploy to cloud environments.

---

## 1. Role-Based Execution from a Single Container Image

The multi-stage `Dockerfile` compiles three distinct binaries into the same runtime image:
- **`judge-ma-di`**: Monolith running both Axum HTTP API and judge worker in a single process (default).
- **`judge-api`**: Standalone HTTP API service (task management, manifest queries, testcase uploads, healthcheck).
- **`judge-worker`**: Standalone evaluation worker polling PostgreSQL for queued submissions.

Because `entrypoint.sh` executes `exec "$@"`, overriding the container command (`CMD`) switches roles without rebuilding the image:

```yaml
command: ["./judge-api"]     # API only
command: ["./judge-worker"]  # Worker only
```

---

## 2. Horizontal Worker Scaling with Docker Compose

To scale evaluation throughput locally or on a single multi-core host, decouple the API service from N parallel worker instances:

```yaml
services:
  db:
    image: postgres:17
    environment:
      POSTGRES_USER: postgres
      POSTGRES_PASSWORD: postgres
      POSTGRES_DB: postgres
    ports:
      - "5432:5432"
    volumes:
      - pgdata:/var/lib/postgresql/data
      - ./scripts/init.sql:/docker-entrypoint-initdb.d/init.sql:ro

  api:
    image: rustapp
    build: .
    command: ["./judge-api"]
    ports:
      - "5000:5000"
    environment:
      POSTGRES_URL: postgres://postgres:postgres@db:5432/postgres
      APP_ENV: production
    volumes:
      - tasks_data:/user/local/bin/tasks
    depends_on:
      - db

  worker:
    image: rustapp
    command: ["./judge-worker"]
    privileged: true
    cgroup: host
    environment:
      POSTGRES_URL: postgres://postgres:postgres@db:5432/postgres
      APP_ENV: production
    volumes:
      - tasks_data:/user/local/bin/tasks:ro
    deploy:
      replicas: 3
    depends_on:
      - db

volumes:
  pgdata:
  tasks_data:
```

### Architecture Guarantees
1. **Atomic Job Dispatch**: Workers poll PostgreSQL using `FOR UPDATE SKIP LOCKED`. PostgreSQL atomically locks and claims rows in FIFO order, guaranteeing zero duplicate evaluation and zero lock contention across workers.
2. **Sandbox Isolation**: Each worker container manages its own local `/var/local/lib/isolate` sandbox root. Isolate box IDs are scoped per-container, preventing filesystem or process collision across worker instances.
3. **Shared Storage**: The `tasks_data` volume is mounted read-write by `api` and read-only (`:ro`) by `worker` replicas, ensuring consistent access to problem manifests, testcases, and checker binaries.

---

## 3. Cloud Deployment & Auto-Scaling

### Sandbox Platform Constraints

IOI Isolate relies on:
- Linux **cgroups v2** (`isolate --cg`) for deterministic CPU time and peak memory accounting.
- Kernel subtree delegation (`entrypoint.sh`) to enable child cgroup controllers.

> [!WARNING]
> **Serverless MicroVM Incompatibility:**
> Serverless container runtimes (such as Google Cloud Run and AWS Fargate) execute containers inside hypervisor sandboxes (gVisor / Firecracker) that disallow cgroup controller delegation. `isolate --cg` fails on these platforms.
>
> **Recommended Platforms:**
> - **Kubernetes**: Google Kubernetes Engine (GKE) or AWS EKS on standard Linux worker nodes with `privileged: true` or container cgroup mounts.
> - **Virtual Machines**: Ubuntu / Debian instances on AWS EC2, GCP Compute Engine, or bare-metal clusters managed with VM autoscaling groups.

---

### Autoscaling Strategy

#### A. HTTP API Scaling
- **Metric**: CPU utilization or incoming HTTP request rate.
- **Mechanism**: Standard Kubernetes Horizontal Pod Autoscaler (HPA) or cloud Application Load Balancer target tracking.
- The API is stateless; instances can be added or terminated immediately.

#### B. Worker Autoscaling (Queue-Depth Driven)
- **Metric**: Depth of unassigned submissions in PostgreSQL:
  ```sql
  SELECT count(*)::int FROM submission WHERE status = 'In Queue';
  ```
- **Mechanism**: **[KEDA](https://keda.sh/)** (Kubernetes Event-driven Autoscaling) using the PostgreSQL trigger:

```yaml
apiVersion: keda.sh/v1alpha1
kind: ScaledObject
metadata:
  name: judge-worker-scaler
spec:
  scaleTargetRef:
    name: judge-worker
  minReplicaCount: 1     # Maintain a warm worker to eliminate cold-start latency
  maxReplicaCount: 10    # Bound by host CPU core count to prevent CPU throttling
  triggers:
  - type: postgresql
    metadata:
      connectionFromEnv: POSTGRES_URL
      query: "SELECT count(*)::int FROM submission WHERE status = 'In Queue'"
      targetQueryValue: "2"  # Scale 1 worker replica per 2 pending submissions
```

During idle periods, worker replicas scale down to `minReplicaCount`. During contest spikes, KEDA provisions additional worker pods to drain the queue.

---

### Task Storage Strategies by Scale

| Scale | Storage Mechanism | Implementation |
| :--- | :--- | :--- |
| **Small to Medium** (<20 workers) | **Shared Network Filesystem** (AWS EFS, GCP Filestore, NFS) | Mount directly to `/user/local/bin/tasks`. Zero code modification required; all workers read testcases directly over NFS. |
| **High Throughput** (>20 workers) | **Object Storage + Worker Local Disk Cache** (AWS S3, Google Cloud Storage) | API writes packages to object storage; workers download and cache problem directories locally upon receiving their first submission for that task. Eliminates shared network disk I/O bottlenecks. |
