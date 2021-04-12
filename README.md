# XIU
XIU is a live server written by RUST.


## Functionalities

- [x] RTMP 
- [ ] HTTPFLV
- [ ] HLS
- ...


## Change Logs

[2021-04-11]

- Impl: add flush\_timeout and read\_timeout functinos.
- Impl: add some log print configuratinos.
- Fix bug: Chunk header with the save csid is not saved.
- Impl: add unsubscribe and unpublish logic in server\_session and channels models.

[2021-04-10]
- Improve: change use libraries format.

[2021-04-09]
- Update README.

[2021-04-08]
- Fix: use mpsc channel to replace onshot,may be improved.

