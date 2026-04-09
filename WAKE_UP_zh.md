**keywords_raw.txt**

```txt
你好玉米糊 @你好玉米糊
嘿玉米糊 @嘿玉米糊
玉米糊在吗 @玉米糊在吗
玉米糊 @玉米糊
```

**转换**

```shell
sherpa-onnx-cli text2token --tokens .\models\wakeword\tokens.txt --tokens-type ppinyin models/wakeword/keywords_raw.txt keywords.txt
```

**示例**
```shell
 cargo run --example wakeup -- --encoder models/wakeword/encoder.onnx --decoder models/wakeword/decoder.onnx --joiner models/wakeword/joiner.onnx --tokens models/wakeword/tokens.txt --keywords models/wakeword/keywords.txt
```