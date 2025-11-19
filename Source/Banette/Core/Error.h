// Copyright 2019-Present tarnishablec. All Rights Reserved.

#pragma once

#include "CoreMinimal.h"
#include "Definition.h"
#include "Experimental/UnifiedError/UnifiedError.h"


namespace Banette
{
	namespace Error
	{
		template <typename T>
		concept CUnifiedError = std::is_base_of_v<UE::UnifiedError::FError, T>;
	}

	template <typename ValueT, Error::CUnifiedError ErrorT = UE::UnifiedError::FError>
	using TResult = TValueOrError<ValueT, ErrorT>;
}
