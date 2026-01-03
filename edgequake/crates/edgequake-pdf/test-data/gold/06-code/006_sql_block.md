# SQL Code Block

```sql
SELECT users.name, COUNT(posts.id) as post_count
FROM users
LEFT JOIN posts ON users.id = posts.user_id
WHERE users.active = true
GROUP BY users.id
ORDER BY post_count DESC;
```

SQL query example above.
