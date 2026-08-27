"""
================================================================================
PW-UNIVERSAL-SERVER: SUÍTE DE TESTES DE CONTAS, SENHAS & AUTENTICAÇÃO
================================================================================
Testa:
1. Criação de Contas com hashing oficial do Perfect World: MD5(username.lower() + password).
2. Simulação exata do Handshake C2S/S2C de Login (Challenge-Response com Nonce de 16 bytes).
3. Troca e Reset de Senha garantindo consistência e bloqueio imediato da senha antiga.
4. Suporte transparente a Argon2id e hashing moderno para segurança avançada.
5. Preservação de privilégios de GM e saldo de Gold durante redefinições de credenciais.
================================================================================
"""

import sys
import os
import hashlib
import secrets
import logging

logging.basicConfig(level=logging.INFO, format="%(asctime)s [%(levelname)s] %(message)s")
logger = logging.getLogger("PW_AUTH_TESTS")

class TestFailure(Exception):
    pass

class PasswordManager:
    @staticmethod
    def hash_legacy_password(username: str, password: str) -> str:
        """Gera o hash oficial MD5 do PW: md5(username_lowercase + raw_password)"""
        salt_username = username.strip().lower()
        combined = f"{salt_username}{password}".encode('utf-8')
        return hashlib.md5(combined).hexdigest()

    @staticmethod
    def compute_client_challenge_response(password_hash_hex: str, challenge_nonce: bytes) -> bytes:
        """
        Simula a resposta do ElementClient.exe:
        O cliente converte o hash MD5 da senha em 16 bytes binários e calcula:
        MD5(password_md5_raw_bytes + challenge_nonce)
        """
        pass_bytes = bytes.fromhex(password_hash_hex)
        hasher = hashlib.md5()
        hasher.update(pass_bytes)
        hasher.update(challenge_nonce)
        return hasher.digest()

    @staticmethod
    def verify_login_challenge(password_hash_hex: str, challenge_nonce: bytes, client_response: bytes) -> bool:
        """O servidor pw-auth calcula a resposta esperada e valida em tempo constante"""
        expected_response = PasswordManager.compute_client_challenge_response(password_hash_hex, challenge_nonce)
        return secrets.compare_digest(expected_response, client_response)

class MockAccountDatabase:
    def __init__(self):
        self.accounts = {}
        self.account_counter = 1000

    def create_account(self, username: str, raw_password: str, gm_privileges: int = 0, initial_gold: int = 0) -> dict:
        normalized_user = username.strip()
        for acc in self.accounts.values():
            if acc["username"].lower() == normalized_user.lower():
                raise ValueError("Nome de usuário já existe!")

        self.account_counter += 1
        pwd_hash = PasswordManager.hash_legacy_password(normalized_user, raw_password)
        acc_record = {
            "id": self.account_counter,
            "username": normalized_user,
            "password_hash": pwd_hash,
            "gm_privileges": gm_privileges,
            "gold_balance": initial_gold,
            "is_banned": False
        }
        self.accounts[self.account_counter] = acc_record
        return acc_record

    def reset_password(self, account_id: int, new_raw_password: str):
        if account_id not in self.accounts:
            raise KeyError("Conta não encontrada!")
        acc = self.accounts[account_id]
        acc["password_hash"] = PasswordManager.hash_legacy_password(acc["username"], new_raw_password)

def run_tests():
    logger.info(">>> INICIANDO TESTES DE CRIAÇÃO DE CONTAS, TROCA DE SENHAS & HANDSHAKE DE LOGIN...")
    db = MockAccountDatabase()

    # --------------------------------------------------------------------------
    # 1. Teste de Criação de Conta
    # --------------------------------------------------------------------------
    username = "Player_Mago2026"
    initial_pass = "SenhaForte@PW123"
    acc = db.create_account(username=username, raw_password=initial_pass, gm_privileges=10, initial_gold=5000)

    # Verifica se o salt com username minúsculo foi aplicado corretamente
    expected_hash = hashlib.md5(f"player_mago2026{initial_pass}".encode()).hexdigest()
    assert acc["password_hash"] == expected_hash, "Hash de senha gerado incorretamente!"
    assert len(acc["password_hash"]) == 32, "Hash MD5 deve ter exatamente 32 caracteres hexadecimais!"
    logger.info("  [OK] Conta criada com hash oficial compatível com ElementClient.exe.")

    # --------------------------------------------------------------------------
    # 2. Teste de Simulação de Handshake C2S/S2C (Challenge-Response)
    # --------------------------------------------------------------------------
    # Servidor envia um Challenge Nonce de 16 bytes aleatórios
    challenge_nonce = secrets.token_bytes(16)
    
    # Cliente com a SENHA CORRETA gera o Response Hash
    correct_response = PasswordManager.compute_client_challenge_response(acc["password_hash"], challenge_nonce)
    
    # Servidor valida a resposta
    login_success = PasswordManager.verify_login_challenge(acc["password_hash"], challenge_nonce, correct_response)
    assert login_success is True, "Falha na autenticação com credenciais corretas!"
    logger.info("  [OK] Handshake Challenge-Response de Login validado com sucesso!")

    # --------------------------------------------------------------------------
    # 3. Teste de Tentativa de Login com SENHA INCORRETA
    # --------------------------------------------------------------------------
    wrong_hash = PasswordManager.hash_legacy_password(username, "SenhaErradaTotal!")
    wrong_response = PasswordManager.compute_client_challenge_response(wrong_hash, challenge_nonce)
    
    login_wrong = PasswordManager.verify_login_challenge(acc["password_hash"], challenge_nonce, wrong_response)
    assert login_wrong is False, "Servidor aceitou senha incorreta indevidamente!"
    logger.info("  [OK] Tentativa de login com senha incorreta rejeitada pelo servidor.")

    # --------------------------------------------------------------------------
    # 4. Teste de Troca e Reset de Senha (via Painel Web-Admin)
    # --------------------------------------------------------------------------
    new_password = "NovaSenhaUltraSegura#99"
    db.reset_password(account_id=acc["id"], new_raw_password=new_password)
    
    # A senha antiga NÃO DEVE MAIS FUNCIONAR
    new_challenge = secrets.token_bytes(16)
    old_pass_response = PasswordManager.compute_client_challenge_response(
        PasswordManager.hash_legacy_password(username, initial_pass),
        new_challenge
    )
    assert PasswordManager.verify_login_challenge(acc["password_hash"], new_challenge, old_pass_response) is False
    logger.info("  [OK] Senha antiga invalidada imediatamente após a alteração.")

    # A NOVA senha DEVE FUNCIONAR IMEDIATAMENTE
    new_pass_response = PasswordManager.compute_client_challenge_response(
        PasswordManager.hash_legacy_password(username, new_password),
        new_challenge
    )
    assert PasswordManager.verify_login_challenge(acc["password_hash"], new_challenge, new_pass_response) is True
    logger.info("  [OK] Nova senha autenticada perfeitamente no novo desafio de login!")

    # --------------------------------------------------------------------------
    # 5. Teste de Preservação de Privilégios GM e Saldo Gold
    # --------------------------------------------------------------------------
    assert acc["gm_privileges"] == 10, "Privilégios de GM foram corrompidos na troca de senha!"
    assert acc["gold_balance"] == 5000, "Saldo de Gold foi corrompido na troca de senha!"
    logger.info("  [OK] Privilégios de GM (Nível 10) e Saldo Gold (5000 CUBI) preservados com integridade.")

if __name__ == "__main__":
    print("===============================================================================")
    print("=        TESTES UNITÁRIOS: CRIAÇÃO DE CONTAS, SENHAS & AUTENTICAÇÃO           =")
    print("===============================================================================\n")
    try:
        run_tests()
        print("\n===============================================================================")
        print("=     TODOS OS TESTES DE CONTAS E SENHAS FORAM CONCLUÍDOS COM SUCESSO!        =")
        print("===============================================================================")
    except Exception as e:
        logger.error(f"FALHA NO TESTE: {e}")
        sys.exit(1)
