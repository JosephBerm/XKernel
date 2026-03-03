"""Tests for xkernal.errors — mirrors Rust error_codes.rs tests."""

from xkernal.errors import CsciError, CsciErrorCode, ErrorCategory


def test_error_code_values():
    assert int(CsciErrorCode.CS_SUCCESS) == 0
    assert int(CsciErrorCode.CS_EPERM) == 1
    assert int(CsciErrorCode.CS_ENOENT) == 2
    assert int(CsciErrorCode.CS_ENOMEM) == 12
    assert int(CsciErrorCode.CS_EBUSY) == 16
    assert int(CsciErrorCode.CS_EEXIST) == 17
    assert int(CsciErrorCode.CS_EINVAL) == 22
    assert int(CsciErrorCode.CS_ETIMEOUT) == 110
    assert int(CsciErrorCode.CS_EBUDGET) == 200
    assert int(CsciErrorCode.CS_ECYCLE) == 201
    assert int(CsciErrorCode.CS_EUNIMPL) == 202


def test_all_20_codes_exist():
    assert len(CsciErrorCode) == 20


def test_error_code_is_success():
    assert CsciErrorCode.CS_SUCCESS.is_success
    assert not CsciErrorCode.CS_EINVAL.is_success
    assert not CsciErrorCode.CS_ENOMEM.is_success


def test_error_code_names():
    assert CsciErrorCode.CS_SUCCESS.short_name == "SUCCESS"
    assert CsciErrorCode.CS_EPERM.short_name == "EPERM"
    assert CsciErrorCode.CS_ENOENT.short_name == "ENOENT"
    assert CsciErrorCode.CS_ENOMEM.short_name == "ENOMEM"
    assert CsciErrorCode.CS_EBUDGET.short_name == "EBUDGET"


def test_error_code_categories():
    assert CsciErrorCode.CS_SUCCESS.category == ErrorCategory.SUCCESS
    assert CsciErrorCode.CS_EPERM.category == ErrorCategory.CAPABILITY
    assert CsciErrorCode.CS_ENOENT.category == ErrorCategory.NOT_FOUND
    assert CsciErrorCode.CS_ENOMEM.category == ErrorCategory.RESOURCE_EXHAUSTION
    assert CsciErrorCode.CS_EBUSY.category == ErrorCategory.RESOURCE_CONFLICT
    assert CsciErrorCode.CS_EEXIST.category == ErrorCategory.RESOURCE_CONFLICT
    assert CsciErrorCode.CS_EINVAL.category == ErrorCategory.INVALID_ARGUMENT
    assert CsciErrorCode.CS_ETIMEOUT.category == ErrorCategory.TIMEOUT
    assert CsciErrorCode.CS_EBUDGET.category == ErrorCategory.RESOURCE_EXHAUSTION
    assert CsciErrorCode.CS_ECYCLE.category == ErrorCategory.LOGIC_ERROR
    assert CsciErrorCode.CS_EUNIMPL.category == ErrorCategory.UNIMPLEMENTED


def test_error_code_descriptions_not_empty():
    for code in CsciErrorCode:
        assert code.description, f"{code.name} has empty description"


def test_csci_error_exception():
    err = CsciError(CsciErrorCode.CS_EINVAL, "bad argument")
    assert err.code == CsciErrorCode.CS_EINVAL
    assert "EINVAL" in str(err)
    assert "bad argument" in str(err)
    assert err.http_status is None


def test_csci_error_with_http_status():
    err = CsciError(CsciErrorCode.CS_ENOENT, "not found", http_status=404)
    assert err.http_status == 404
    assert err.code == CsciErrorCode.CS_ENOENT


def test_convenience_errors():
    from xkernal.errors import NotFoundError, InvalidArgumentError, ChannelClosedError

    e1 = NotFoundError("agent xyz")
    assert e1.code == CsciErrorCode.CS_ENOENT
    assert e1.http_status == 404

    e2 = InvalidArgumentError()
    assert e2.code == CsciErrorCode.CS_EINVAL
    assert e2.http_status == 400

    e3 = ChannelClosedError()
    assert e3.code == CsciErrorCode.CS_ECLOSED
    assert e3.http_status == 410
