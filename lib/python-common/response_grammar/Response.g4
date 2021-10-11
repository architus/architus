grammar Response;

response
    : (respObj | STRING)+
    ;

respObj
    : respList
    | REACT
    | NOUN
    | ADJ
    | ADV
    | OWL
    | COUNT
    | MEMBER
    | AUTHOR
    | CAPTURE
    ;

respList
    : '[' listElement+ response ']'
    ;

listElement
    : response ','
    ;

STRING
    : [A-Za-z0-9]+
    ;

REACT
    : '[<' 'a'? ':' [A-Za-z0-9]+ ':' [0-9]+ '>]'
    ;

NOUN
    : '[noun]'
    | '[NOUN]'
    ;

ADJ
    : '[adj]'
    | '[ADJ]'
    ;

ADV
    : '[adv]'
    | '[ADV]'
    ;

OWL
    : '[owl]'
    | '[OWL]'
    ;

COUNT
    : '[count]'
    | '[COUNT]'
    ;

MEMBER
    : '[member]'
    | '[MEMBER]'
    ;

AUTHOR
    : '[author]'
    | '[AUTHOR]'
    ;

CAPTURE
    : '[capture]'
    | '[CAPTURE]'
    ;

WS
    : [ \t\r\n]+ -> skip
    ;
