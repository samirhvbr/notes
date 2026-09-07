# os mesmos payloads, dentro de codigo

Isto tem de renderizar como TEXTO, nao ser bloqueado nem executado:

```html
<script>window.__pwned = 1</script>
<img src="x" onerror="window.__pwned = 1">
```

E inline: `<script>alert(1)</script>` e `[x](javascript:alert(1))`.
