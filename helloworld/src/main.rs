#![no_main]
use anyhow::Result;
use bitcoin::Transaction;
use bitcoin::consensus;
use risc0_zkvm::guest::env;
use borsh::{BorshDeserialize, BorshSerialize};
use serde::{Deserialize, Serialize};
use std::cell::RefCell;

#[cfg(target_os = "zkvm")]
entrypoint!(handler);

#[cfg(target_os = "zkvm")]
fn handler(
    _program_id: &Pubkey,
    utxos: &[UtxoInfo],
    instruction_data: &[u8],
) -> Result<Vec<u8>> {

    let params: HelloWorldParams = borsh::from_slice(instruction_data)?;

    for utxo in utxos {
        *utxo.data.borrow_mut() = format!("Hello {}!", params.name).as_str().as_bytes().to_vec();
    }

    let mut tx: Transaction = consensus::deserialize(&params.tx_hex).unwrap();
    Ok(consensus::serialize(&tx))
}

#[derive(Clone, BorshSerialize, BorshDeserialize)]
pub struct HelloWorldParams {
    pub name: String,
    pub tx_hex: Vec<u8>,
}

#[macro_export]
macro_rules! entrypoint {
    ($process_instruction:ident) => {
        risc0_zkvm::guest::entry!(entrypoint);
        use std::collections::HashMap;
        use hex;

        pub fn entrypoint() {
            let serialized_instruction: Vec<u8> = env::read();
            let instruction: Instruction = borsh::from_slice(&serialized_instruction).unwrap();
            let program_id: Pubkey = instruction.program_id;
            let authorities: HashMap<String, Vec<u8>> = env::read();
            let data: HashMap<String, Vec<u8>> = env::read();

            let utxos = instruction
                .utxos
                .iter()
                .map(|utxo| {
                    use std::cell::RefCell;

                    UtxoInfo {
                        txid: utxo.txid.clone(),
                        vout: utxo.vout,
                        authority: RefCell::new(Pubkey(
                            authorities
                                .get(&utxo.id())
                                .expect("this utxo does not exist")
                                .to_vec(),
                        )),
                        data: RefCell::new(
                            data.get(&utxo.id())
                                .expect("this utxo does not exist")
                                .to_vec(),
                        ),
                    }
                })
                .collect::<Vec<UtxoInfo>>();

            let instruction_data: Vec<u8> = instruction.data;

            match $process_instruction(&program_id, &utxos, &instruction_data) {
                Ok(tx_hex) => {
                    let mut new_authorities: HashMap<String, Vec<u8>> = HashMap::new();
                    let mut new_data: HashMap<String, Vec<u8>> = HashMap::new();
                    utxos.iter().for_each(|utxo| {
                        new_authorities.insert(utxo.id(), utxo.authority.clone().into_inner().0);
                        new_data.insert(utxo.id(), utxo.data.clone().into_inner());
                    });
                    env::commit(&borsh::to_vec(&(new_authorities, new_data, tx_hex)).unwrap())
                }
                Err(e) => panic!("err: {:?}", e),
            }
        }
    };
}

#[derive(Clone, Debug, Eq, PartialEq, Serialize, Deserialize, BorshSerialize, BorshDeserialize, Default,)]
pub struct Pubkey(pub Vec<u8>);

#[derive(Clone, Debug, Eq, PartialEq, BorshSerialize, BorshDeserialize)]
pub struct UtxoMeta {
    pub txid: String,
    pub vout: u32,
}

impl UtxoMeta {
    pub fn id(&self) -> String {
        format!("{}:{}", self.txid, self.vout)
    }
}

#[derive(Clone, Debug, Eq, PartialEq, BorshSerialize, BorshDeserialize)]
pub struct UtxoInfo {
    pub txid: String,
    pub vout: u32,
    pub authority: RefCell<Pubkey>,
    pub data: RefCell<Vec<u8>>,
}

impl UtxoInfo {
    pub fn id(&self) -> String {
        format!("{}:{}", self.txid, self.vout)
    }
}

#[derive(Clone, Debug, Eq, PartialEq, BorshSerialize, BorshDeserialize)]
pub struct Instruction {
    pub program_id: Pubkey,
    pub utxos: Vec<UtxoMeta>,
    pub data: Vec<u8>,
}
