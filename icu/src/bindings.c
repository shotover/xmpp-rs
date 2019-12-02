// This file is a stupid wrapper to avoid the automated suffixing libicu is
// doing in unicode/urename.h.
//
// By default it will suffix each of its symbols with "_65" (with 65 being the
// soname), which completely messes with Rust’s binding ability.

#include <unicode/umachine.h>
#include <unicode/utypes.h>
#include <unicode/usprep.h>
#include <unicode/utrace.h>
#include <unicode/uidna.h>
#include <unicode/uspoof.h>
#include <unicode/ustring.h>
#include <string.h>

const char* icu_error_code_to_name(UErrorCode code) {
	return u_errorName(code);
}

UIDNA* icu_idna_open(uint32_t options, UErrorCode* pErrorCode) {
	return uidna_openUTS46(options, pErrorCode);
}

int32_t icu_idna_name_to_ascii(const UIDNA* idna, const char* name, int32_t length, char* dest, int32_t capacity, UIDNAInfo* pInfo, UErrorCode* pErrorCode) {
	return uidna_nameToASCII_UTF8(idna, name, length, dest, capacity, pInfo, pErrorCode);
}

int32_t icu_idna_name_to_unicode(const UIDNA* idna, const char* name, int32_t length, char* dest, int32_t capacity, UIDNAInfo* pInfo, UErrorCode* pErrorCode) {
	return uidna_nameToUnicodeUTF8(idna, name, length, dest, capacity, pInfo, pErrorCode);
}

UStringPrepProfile* icu_stringprep_open(UStringPrepProfileType type, UErrorCode* status) {
	return usprep_openByType(type, status);
}

int32_t icu_stringprep_prepare(const UStringPrepProfile* prep, const UChar* src, int32_t srcLength, UChar* dest, int32_t destCapacity, int32_t options, UParseError* parseError, UErrorCode* status) {
	return usprep_prepare(prep, src, srcLength, dest, destCapacity, options, parseError, status);
}

void icu_trace_set_level(UTraceLevel traceLevel) {
	utrace_setLevel(traceLevel);
}

USpoofChecker* icu_spoof_open(UErrorCode* status) {
	return uspoof_open(status);
}

void icu_spoof_set_checks(USpoofChecker* sc, int32_t checks, UErrorCode* status) {
	uspoof_setChecks(sc, checks, status);
}

int32_t icu_spoof_get_skeleton(USpoofChecker* sc, uint32_t type, const char* id, int32_t length, char* dest, int32_t destCapacity, UErrorCode* status) {
	return uspoof_getSkeletonUTF8(sc, type, id, length, dest, destCapacity, status);
}
