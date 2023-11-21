#![cfg_attr(not(feature = "std"), no_std)]

//! # Concordium V1 Smart Contract
use concordium_std::*;
use core::fmt::Debug;

/// Represents the state of the Concordium V1 smart contract.
#[derive(Debug, Serial, DeserialWithState)]
#[concordium(state_parameter = "S")]
pub struct MultiSigContractState<S: HasStateApi = StateApi> {
    pub pending_transactions: StateMap<u32, TransactionProposal, S>,
    pub administrators: StateBox<Vec<Address>, S>,
}

impl MultiSigContractState {
    pub fn num_voters(&self) -> usize {
        self.administrators.len()
    }

    pub fn is_administrator(&self, sender: &Address) -> bool {
        self.administrators.contains(sender)
    }
}

/// Trait defining ownership-related functions.
pub trait IsAdministrator {
    fn is_administrator(&self, host: &Host<MultiSigContractState>, sender: &Address) -> bool {
        host.state().administrators.contains(sender)
    }

    fn num_voters(&self, host: &Host<MultiSigContractState>) -> usize {
        host.state().administrators.len()
    }
}

#[derive(Serialize, SchemaType, Debug, PartialEq, Eq, Clone)]
pub struct TransactionProposal {
    pub index: u32,
    pub amount: Amount,
    pub recipient: AccountAddress,
    pub voters: Vec<Address>,
    pub approvals: u8,
    pub fulfilled: bool,
    pub proposer: Address,
}

impl IsAdministrator for TransactionProposal {}

impl TransactionProposal {
    pub fn new(index: u32, amount: Amount, recipient: AccountAddress, proposer: Address) -> Self {
        TransactionProposal {
            index,
            amount,
            recipient,
            voters: Vec::new(),
            approvals: 0,
            fulfilled: false,
            proposer,
        }
    }

    pub fn approve(&mut self, ctx: &ReceiveContext, required_approvals: usize) -> Result<bool, Error> {
        if self.voters.contains(&ctx.sender()) {
            return Err(Error::AlreadyVoted);
        } else {
            self.voters.push(ctx.sender());
            self.approvals += 1;
            Ok(self.approvals == required_approvals as u8)
        }
    }

    pub fn is_approved(&self, required_approvals: usize) -> Result<bool, Error> {
        Ok(self.approvals == required_approvals as u8)
    }
}

impl MultiSigContractState {
    pub fn new(state_builder: &mut StateBuilder, administrators: Vec<Address>) -> Self {
        MultiSigContractState {
            pending_transactions: state_builder.new_map(),
            administrators: state_builder.new_box(administrators),
        }
    }
}

/// Enum representing errors in the smart contract.
#[derive(Debug, PartialEq, Eq, Reject, Serialize, SchemaType)]
pub enum Error {
    #[from(ParseError)]
    ParseParams,
    YourError,
    AlreadyVoted,
    TransactionNotApprovedOrFulfilled,
    TransactionKeyAlreadyExists,
}

#[derive(Serialize, SchemaType, Debug, PartialEq, Eq)]
pub struct InitializationParams {
    pub administrators: Vec<Address>,
}

#[derive(Serialize, SchemaType)]
pub struct TransactionParams {
    pub index: u32,
    pub receiver: AccountAddress,
    pub amount: Amount,
}

impl TransactionParams {
    pub fn default() -> Self {
        TransactionParams {
            index: 0,
            receiver: AccountAddress([0u8; 32]),
            amount: Amount { micro_ccd: 0 },
        }
    }

    pub fn new(index: u32, receiver: AccountAddress, amount: u64) -> Self {
        TransactionParams {
            index,
            receiver,
            amount: Amount { micro_ccd: amount },
        }
    }
}

#[derive(Serialize, SchemaType, Debug, PartialEq, Eq )]
pub struct ApprovalParams {
    pub index: u32,
}

impl ApprovalParams {
    pub fn new(index: u32) -> Self {
        ApprovalParams { index }
    }
}

// creates new smart contract
#[init(contract = "ccd_multisig", parameter = "InitializationParams")]
fn initialize(ctx: &InitContext, state_builder: &mut StateBuilder) -> InitResult<MultiSigContractState> {
    let params: InitializationParams = ctx.parameter_cursor().get()?;
    let administrators = params.administrators;
    let state = MultiSigContractState::new(state_builder, administrators);
    Ok(state)
}

#[receive(contract = "ccd_multisig", name = "transfer", parameter = "ApprovalParams", mutable)]
fn transfer(ctx: &ReceiveContext, host: &mut Host<MultiSigContractState>) -> ReceiveResult<()> {
    let param:ApprovalParams = ctx.parameter_cursor().get()?;
    let required_approvals = host.state().administrators.len();
    let index = param.index;
    let approved = host.state_mut()
        .pending_transactions
        .get(&index).unwrap().is_approved(required_approvals).unwrap();
    let not_fulfilled = host.state_mut()
        .pending_transactions
        .get_mut(&index).unwrap().fulfilled == false;
    let amount = host.state_mut()
        .pending_transactions
        .get(&index).unwrap().amount;
    let recipient = host.state_mut()
        .pending_transactions
        .get(&index).unwrap().recipient;
    if host.self_balance() < amount {
        bail!()
    }
    match (approved,not_fulfilled) {
        (true,true) => {
            host.state_mut()
            .pending_transactions
            .get_mut(&index).unwrap().fulfilled = true;
            let response = host.invoke_transfer(&recipient, amount);
            match response{
                Ok(()) => Ok(()),
                Err(_) => bail!()
            }
        },
        _ => bail!()
    }
}

// recieves CCD from anybody
#[receive(contract = "ccd_multisig", name = "insert", payable)]
#[allow(unused_variables)]
fn insert(ctx: &ReceiveContext, _host: &Host<MultiSigContractState>, _amount: Amount) -> ReceiveResult<()> {
    Ok(())
}

/// initialises a new transaction pending approval
#[receive(contract = "ccd_multisig", name = "create_transaction", parameter = "TransactionParams", mutable)]
pub fn create_transaction(
    ctx: &ReceiveContext,
    host: &mut Host<MultiSigContractState>,
) -> Result<u32, Error> {
    let param:TransactionParams = ctx.parameter_cursor().get()?;
    if let None = host.state().pending_transactions.get(&param.index){
        let proposal = TransactionProposal::new(param.index,param.amount,param.receiver,0,ctx.sender());
            host.state_mut().pending_transactions.insert(param.index, proposal);
            Ok(param.index)
    }else{
        Err(Error::TransactionKeyAlreadyExists)        
    }
}

#[receive(contract = "ccd_multisig", name = "approve", parameter = "ApprovalParams", mutable)]
pub fn approve(
    ctx: &ReceiveContext,
    host: &mut Host<MultiSigContractState>,
) -> ReceiveResult<bool> {
    let param:ApprovalParams = ctx.parameter_cursor().get()?;
    let index = param.index;
    if host.state().is_administrator(&ctx.sender()){
        let required_approvals = host.state().num_voters();
        let mut proposal = host.state_mut().pending_transactions.get_mut(&index)
            .expect("The key does not exist");
        ensure_eq!(index,proposal.index);
        let approved = proposal.approve(ctx,required_approvals)?;
        Ok(approved)
    }else{
        bail!()
    }
}

#[receive(contract = "ccd_multisig", name = "view", parameter = "ApprovalParams", return_value = "TransactionProposal")]
fn view<'a, 'b>(
    ctx: &'a ReceiveContext,
    host: &'b Host<MultiSigContractState>,
) -> ReceiveResult<TransactionProposal> {
    let param:ApprovalParams = ctx.parameter_cursor().get()?;
    let prop = host.state().pending_transactions.get(&param.index).unwrap();
    let mut voters = Vec::new();
    prop.voters.iter().for_each(|i| voters.push(*i));
    let (index,amount,recipient, approvals,fulfilled, proposer) = (prop.index,prop.amount,prop.recipient,prop.approvals,prop.fulfilled,prop.proposer);
    Ok(TransactionProposal{index,amount,recipient,voters,approvals,fulfilled,proposer})
}

#[receive(
    contract = "ccd_multisig",
    name = "get_administrators",
    parameter = "ApprovalParams",
    return_value = "Vec<Address>"
)]
fn get_administrators<'a, 'b>(
    _ctx: &'a ReceiveContext,
    host: &'b Host<MultiSigContractState>,
) -> ReceiveResult<Vec<Address>> {
    let admins = host.state().administrators.clone();
    let mut voters = Vec::new();
    admins.iter().for_each(|admin| voters.push(*admin));
    Ok(voters)
}

#[receive(
    contract = "ccd_multisig",
    name = "get_approvals_remaining",
    parameter = "ApprovalParams",
    return_value = "u8"
)]
fn get_approvals_remaining<'a, 'b>(
    ctx: &'a ReceiveContext,
    host: &'b Host<MultiSigContractState>,
) -> ReceiveResult<u8> {
    let admins = host.state().administrators.clone();
    let param:ApprovalParams = ctx.parameter_cursor().get()?;
    let proposal = host.state().pending_transactions.get(&param.index).unwrap();
    Ok(admins.len() as u8 - proposal.approvals)
}
