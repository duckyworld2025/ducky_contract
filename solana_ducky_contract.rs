use solana_program::{
    account_info::{next_account_info, AccountInfo},
    entrypoint,
    entrypoint::ProgramResult,
    msg,
    pubkey::Pubkey,
    program::invoke_signed,
    program_error::ProgramError, // ProgramError를 명시적으로 사용
};
use spl_token::instruction::{mint_to, transfer};

//프로그램의 진입점 (Entrypoint)을 정의합니다.
entrypoint!(process_instruction);

//인스트럭션 유형을 정의합니다. (기존과 동일)
pub enum TokenInstruction {
    //토큰을 민트하여 사용자 계정에 발행합니다.
    MintTokens { amount: u64 },
    
    //토큰을 한 계정에서 다른 계정으로 전송합니다.
    TransferTokens { amount: u64 },
}

impl TokenInstruction {
    pub fn unpack(input: &[u8]) -> Result<Self, ProgramError> {
        let (&tag, rest) = input.split_first().ok_or(ProgramError::InvalidInstructionData)?;
        Ok(match tag {
            0 => {
                let amount = rest
                    .get(..8)
                    .and_then(|slice| slice.try_into().ok())
                    .map(u64::from_le_bytes)
                    .ok_or(ProgramError::InvalidInstructionData)?;
                Self::MintTokens { amount }
            }
            1 => {
                let amount = rest
                    .get(..8)
                    .and_then(|slice| slice.try_into().ok())
                    .map(u64::from_le_bytes)
                    .ok_or(ProgramError::InvalidInstructionData)?;
                Self::TransferTokens { amount }
            }
            _ => return Err(ProgramError::InvalidInstructionData),
        })
    }
}


// 메인 인스트럭션 처리 함수 
pub fn process_instruction(
    program_id: &Pubkey,
    accounts: &[AccountInfo],
    instruction_data: &[u8],
) -> ProgramResult {
    let instruction = TokenInstruction::unpack(instruction_data)?;

    match instruction {
        TokenInstruction::MintTokens { amount } => {
            msg!("Instruction: Mint Tokens");
            mint_tokens(program_id, accounts, amount)
        }
        TokenInstruction::TransferTokens { amount } => {
            msg!("Instruction: Transfer Tokens");
            transfer_tokens(program_id, accounts, amount)
        }
    }
}

/// 💸 토큰 발행 (Mint To) 로직 **(수정됨)**
pub fn mint_tokens(
    _program_id: &Pubkey,
    accounts: &[AccountInfo],
    amount: u64,
) -> ProgramResult {
    let account_info_iter = &mut accounts.iter();

    // 필요한 계정 정의 (클라이언트에서 순서대로 전달해야 합니다)
    let mint_account = next_account_info(account_info_iter)?; // 1. 토큰 민트 (Mint) 계정
    let token_account = next_account_info(account_info_iter)?; // 2. 토큰을 받을 SPL 토큰 계정
    // 3. 민트 권한을 가진 계정 (이 계정이 트랜잭션 서명을 제공해야 합니다)
    let mint_authority = next_account_info(account_info_iter)?; 
    let spl_token_program = next_account_info(account_info_iter)?; // 4. SPL 토큰 프로그램 ID

    
    // 민트 권한을 가진 계정이 트랜잭션에 서명했는지 확인합니다.
    if !mint_authority.is_signer {
        msg!("Mint Authority must be a signer for this transaction.");
        return Err(ProgramError::MissingRequiredSignature);
    }
    // -----------------------------------------------------------
    
    // 1. SPL `mint_to` 인스트럭션 생성
    let mint_instruction = mint_to(
        spl_token_program.key,
        mint_account.key,
        token_account.key,
        mint_authority.key,
        &[], // 다중 서명자가 있는 경우 여기에 추가
        amount,
    )?;

    // 2. SPL 프로그램 호출 (Invoke)
    invoke_signed(
        &mint_instruction,
        &[
            mint_account.clone(),
            token_account.clone(),
            mint_authority.clone(),
            spl_token_program.clone(),
        ],
        &[], // 서명자 시드 (PDA가 서명할 경우 사용)
    )?;

    msg!("Successfully minted {} tokens to {}", amount, token_account.key);

    Ok(())
}

/// 📦 토큰 전송 (Transfer) 로직 (기존과 동일)
pub fn transfer_tokens(
    _program_id: &Pubkey,
    accounts: &[AccountInfo],
    amount: u64,
) -> ProgramResult {
    let account_info_iter = &mut accounts.iter();

    // 필요한 계정 정의 (클라이언트에서 순서대로 전달해야 합니다)
    let source_account = next_account_info(account_info_iter)?; // 1. 토큰을 보낼 SPL 토큰 계정
    let destination_account = next_account_info(account_info_iter)?; // 2. 토큰을 받을 SPL 토큰 계정
    let owner_account = next_account_info(account_info_iter)?; // 3. 소스 계정의 소유자 (서명자)
    let _mint_account = next_account_info(account_info_iter)?; // 4. 토큰 민트 (검증용)
    let spl_token_program = next_account_info(account_info_iter)?; // 5. SPL 토큰 프로그램 ID

    // 전송 권한 확인: 소유자 계정이 서명했는지 확인합니다.
    if !owner_account.is_signer {
        msg!("Token source owner must be a signer to transfer.");
        return Err(ProgramError::MissingRequiredSignature);
    }

    // 1. SPL `transfer` 인스트럭션 생성
    let transfer_instruction = transfer(
        spl_token_program.key,
        source_account.key,
        destination_account.key,
        owner_account.key,
        &[], // 다중 서명자가 있는 경우 여기에 추가
        amount,
    )?;

    // 2. SPL 프로그램 호출 (Invoke)
    invoke_signed(
        &transfer_instruction,
        &[
            source_account.clone(),
            destination_account.clone(),
            owner_account.clone(),
            spl_token_program.clone(),
        ],
        &[], // 서명자 시드 (PDA가 서명할 경우 사용)
    )?;

    msg!("Successfully transferred {} tokens from {} to {}", amount, source_account.key, destination_account.key);

    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_unpack_mint_tokens() {
        let amount: u64 = 1000;
        let mut instruction_data = Vec::new();
        instruction_data.push(0); // Tag 0: MintTokens
        instruction_data.extend_from_slice(&amount.to_le_bytes());

        let instruction = TokenInstruction::unpack(&instruction_data).unwrap();
        
        match instruction {
            TokenInstruction::MintTokens { amount: unpacked_amount } => {
                assert_eq!(unpacked_amount, amount);
            }
            _ => panic!("Expected MintTokens instruction"),
        }
    }
}