# BITE State-Machine Prototype

This offline prototype models trade-state transitions only. It has no wallet, cryptography, network, relay, Bitcoin node, PSBT parser, key management, or transaction broadcast functionality.

It must not be deployed as an exchange or used to handle real funds. Events such as signature-policy satisfaction and chain observations are trusted inputs in this model; production integration must verify them at the relevant trust boundary before calling the transition function.

Run with Node.js 22 or later:

```sh
npm test
```
