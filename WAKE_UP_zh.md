**keywords_raw.txt**

```txt
你好玉米糊 @你好玉米糊
嘿玉米糊 @嘿玉米糊
玉米糊在吗 @玉米糊在吗
玉米糊 @玉米糊
```

```shell
sherpa-onnx-cli text2token --tokens .\models\wakeword\tokens.txt --tokens-type ppinyin models/wakeword/keywords_raw.txt keywords.txt
```