"""Sanitized extension-hook/default propagation regression fixture."""
import unittest


class Signer:
    default_salt = "application-default"

    def make_signer(self, salt=None):
        return self.default_salt if salt is None else salt

    def sign(self, salt=None):
        return self.make_signer(salt)


class RefactoredSigner(Signer):
    def sign(self, salt=None):
        # The base implementation tests pass, but this bypasses overrides and
        # resolves the default before an extension hook can inspect None.
        return Signer.make_signer(self, salt or self.default_salt)


class Extension:
    def make_signer(self, salt=None):
        return ("override", salt)


class ActiveExtension(Extension, Signer):
    pass


class BaseContract(unittest.TestCase):
    def test_default_and_explicit_salt(self):
        for signer in (Signer(), RefactoredSigner()):
            self.assertEqual(signer.sign(), "application-default")
            self.assertEqual(signer.sign("custom"), "custom")


class ExtensionContract(unittest.TestCase):
    def test_override_receives_original_default(self):
        self.assertEqual(ActiveExtension().sign(), ("override", None))

    def test_override_receives_explicit_salt(self):
        self.assertEqual(ActiveExtension().sign("custom"), ("override", "custom"))

    def test_empty_salt_is_not_replaced(self):
        self.assertEqual(ActiveExtension().sign(""), ("override", ""))


if __name__ == "__main__":
    unittest.main()
