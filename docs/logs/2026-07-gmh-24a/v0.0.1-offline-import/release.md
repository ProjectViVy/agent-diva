# GMH-24A Release

This is an offline, explicitly invoked migration surface. It does not switch
production reads or writes. Existing installations without a Memory authority
setting remain on `legacy`.

No push or production deployment is performed. GMH-24B owns shadow/read
cutover; GMH-24C owns typed write cutover and Mentle deletion.
