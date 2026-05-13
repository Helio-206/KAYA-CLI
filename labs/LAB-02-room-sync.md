# LAB-02: Room Sync

Goal: verify public room message routing.

## Steps

Terminal 1:

```text
> /join semana-info
> alguem recebe?
```

Terminal 2:

```text
> /join semana-info
> recebido
```

## Expected Result

Both terminals show traffic in `#semana-info`.

```text
[#semana-info] Helio: alguem recebe?
[#semana-info] Ana: recebido
```

## Notes

Rooms are created locally when joined or when a packet references them. There is no central room registry.
