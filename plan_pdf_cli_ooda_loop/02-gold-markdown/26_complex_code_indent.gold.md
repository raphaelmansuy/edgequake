# Code Indentation Test

## Python with Nested Structures

```python
class DataProcessor:
    def __init__(self, config: dict):
        self.config = config
        self.results = []
    
    def process(self, data: list) -> list:
        for item in data:
            if item.get("type") == "A":
                result = self._process_type_a(item)
                if result:
                    for sub in result:
                        self.results.append({
                            "source": item,
                            "processed": sub,
                            "metadata": {
                                "timestamp": time.now(),
                                "version": "1.0"
                            }
                        })
        return self.results
```

## YAML Configuration

```yaml
server:
  host: localhost
  port: 8080
  ssl:
    enabled: true
    certificate: /path/to/cert.pem
    key: /path/to/key.pem
  
database:
  connections:
    - name: primary
      host: db1.example.com
      port: 5432
    - name: replica
      host: db2.example.com
      port: 5432
```
